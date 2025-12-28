mod card;
mod ime_input;
mod select_option;
mod selectbox;
mod single_line;
mod single_svg;
mod text_input;
mod textedit;
mod view_element_state;

use std::{cmp::Ordering, sync::mpsc::Receiver};

pub use card::Card;
pub use ime_input::ImeInput;
pub use select_option::SelectOption;
pub use selectbox::SelectBox;
pub use single_line::SingleLine;
pub use single_svg::SingleSvg;
pub use text_input::TextInput;
pub use textedit::TextEdit;

use font_rasterizer::color_theme::{ColorTheme, ThemedColor};
use text_buffer::{buffer::BufferChar, caret::CaretType, editor::ChangeEvent};

fn get_color(color_theme: &ColorTheme, c: char) -> [f32; 3] {
    if c.is_ascii() {
        color_theme.yellow().get_color()
    } else if ('あ'..'一').contains(&c) {
        color_theme.text().get_color()
    } else if c < '\u{1F600}' {
        color_theme.cyan().get_color()
    } else {
        color_theme.green().get_color()
    }
}

enum Pos {
    First(char),
    Center(char),
    Last(char),
}

pub fn split_preedit_string(
    value: String,
    start_bytes: usize,
    end_bytes: usize,
) -> (String, String, String) {
    let splitted = value
        .chars()
        .scan(0_usize, |prev, c| {
            *prev += c.len_utf8();
            let prev = *prev;
            if prev <= start_bytes {
                Some(Pos::First(c))
            } else if prev <= end_bytes {
                Some(Pos::Center(c))
            } else {
                Some(Pos::Last(c))
            }
        })
        .collect::<Vec<_>>();
    let first: String = splitted
        .iter()
        .flat_map(|p| if let Pos::First(c) = p { Some(c) } else { None })
        .collect();
    let center: String = splitted
        .iter()
        .flat_map(|p| {
            if let Pos::Center(c) = p {
                Some(c)
            } else {
                None
            }
        })
        .collect();
    let last: String = splitted
        .iter()
        .flat_map(|p| if let Pos::Last(c) = p { Some(c) } else { None })
        .collect();
    (first, center, last)
}

#[inline]
pub fn caret_char(caret_type: CaretType) -> char {
    match caret_type {
        CaretType::Primary => '_',
        CaretType::Mark => '^',
    }
}

#[inline]
pub fn ime_chars() -> [char; 2] {
    ['[', ']']
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum SortOrder {
    Ascending,
    Descending,
    Unsorted,
}

pub(crate) fn detect_sort_order<T: Ord>(items: &[T]) -> SortOrder {
    if items.len() <= 1 {
        return SortOrder::Ascending;
    }

    let mut direction: Option<Ordering> = None;

    for pair in items.windows(2) {
        let cmp = pair[0].cmp(&pair[1]);
        match cmp {
            Ordering::Equal => continue,
            Ordering::Less | Ordering::Greater => {
                if let Some(dir) = direction {
                    let valid = match dir {
                        Ordering::Less => cmp != Ordering::Greater,
                        Ordering::Greater => cmp != Ordering::Less,
                        Ordering::Equal => true,
                    };
                    if !valid {
                        return SortOrder::Unsorted;
                    }
                } else {
                    direction = Some(cmp);
                }
            }
        }
    }

    match direction {
        Some(Ordering::Less) | None => SortOrder::Ascending,
        Some(Ordering::Greater) => SortOrder::Descending,
        Some(Ordering::Equal) => SortOrder::Ascending,
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub enum Decoration {
    None,
    Bold,
    Italic,
    Underline,
    Strikethrough,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub struct CharAttribute {
    pub color: ThemedColor,
    pub decoration: Decoration,
}

pub const DEFAULT_CHAR_ATTRIBUTE: CharAttribute = CharAttribute {
    color: ThemedColor::Text,
    decoration: Decoration::None,
};

impl Default for CharAttribute {
    fn default() -> Self {
        DEFAULT_CHAR_ATTRIBUTE
    }
}

impl CharAttribute {
    pub fn new(color: ThemedColor, decoration: Decoration) -> Self {
        Self { color, decoration }
    }
}

#[derive(Debug)]
pub(crate) enum BulkedChangeEvent {
    SingleEvent(ChangeEvent),
    // ChangeEvent::MoveChar { from, to } の中身だけ受け付ける必要があるので from, to の組を保持する
    MultipleEvents(Vec<(BufferChar, BufferChar)>),
}

pub(crate) fn bulk_change_events(receiver: &Receiver<ChangeEvent>) -> Vec<BulkedChangeEvent> {
    let mut bulked_events = Vec::new();
    let mut buffered_move_char = Vec::new();

    fn flush(
        bulked_events: &mut Vec<BulkedChangeEvent>,
        buffered_move_char: &mut Vec<(BufferChar, BufferChar)>,
    ) {
        if buffered_move_char.is_empty() {
            return;
        }
        let events = std::mem::take(buffered_move_char);
        bulked_events.push(BulkedChangeEvent::MultipleEvents(events));
    }

    for event in receiver.try_iter() {
        match event {
            ChangeEvent::MoveChar { from, to } => {
                buffered_move_char.push((from, to));
            }
            _ => {
                flush(&mut bulked_events, &mut buffered_move_char);
                bulked_events.push(BulkedChangeEvent::SingleEvent(event));
            }
        }
    }
    flush(&mut bulked_events, &mut buffered_move_char);
    bulked_events
}

#[cfg(test)]
mod test {
    use std::sync::mpsc::channel;

    use super::{
        BulkedChangeEvent, SortOrder, bulk_change_events, detect_sort_order, split_preedit_string,
    };
    use text_buffer::{buffer::BufferChar, editor::ChangeEvent};

    #[test]
    fn test_split1() {
        test_split("こんにちは", 6, 12, ("こん", "にち", "は"));
        test_split("こんにちは", 0, 12, ("", "こんにち", "は"));
        test_split("こんにちは", 0, 15, ("", "こんにちは", ""));
        test_split("ABCDE", 2, 3, ("AB", "C", "DE"));
        test_split("AあBいCう", 4, 8, ("Aあ", "Bい", "Cう"));
    }

    fn test_split(target: &str, start: usize, end: usize, expects: (&str, &str, &str)) {
        let (first, center, last) = split_preedit_string(target.to_string(), start, end);
        assert_eq!(&first, expects.0);
        assert_eq!(&center, expects.1);
        assert_eq!(&last, expects.2);
    }

    #[test]
    fn detect_sort_order_cases() {
        assert_eq!(detect_sort_order::<i32>(&[]), SortOrder::Ascending);
        assert_eq!(detect_sort_order(&[1]), SortOrder::Ascending);
        assert_eq!(detect_sort_order(&[1, 2, 2, 3]), SortOrder::Ascending);
        assert_eq!(detect_sort_order(&[5, 4, 4, 1]), SortOrder::Descending);
        assert_eq!(detect_sort_order(&[1, 3, 2]), SortOrder::Unsorted);
    }

    fn bc(row: usize, col: usize, c: char) -> BufferChar {
        BufferChar {
            position: [row, col].into(),
            c,
        }
    }

    #[test]
    fn bulk_change_groups_consecutive_move_events() {
        let (tx, rx) = channel::<ChangeEvent>();

        let from1 = bc(0, 0, 'a');
        let to1 = bc(0, 1, 'a');
        let from2 = bc(1, 0, 'b');
        let to2 = bc(1, 1, 'b');
        let add_char = bc(2, 0, 'c');

        tx.send(ChangeEvent::MoveChar {
            from: from1,
            to: to1,
        })
        .unwrap();
        tx.send(ChangeEvent::MoveChar {
            from: from2,
            to: to2,
        })
        .unwrap();
        tx.send(ChangeEvent::AddChar(add_char)).unwrap();

        let events = bulk_change_events(&rx);
        assert_eq!(events.len(), 2);

        match &events[0] {
            BulkedChangeEvent::MultipleEvents(buffered) => {
                assert_eq!(buffered, &vec![(from1, to1), (from2, to2)]);
            }
            other => panic!("expected MultipleEvents, got {:?}", other),
        }

        match &events[1] {
            BulkedChangeEvent::SingleEvent(ChangeEvent::AddChar(c)) => {
                assert_eq!(c, &add_char);
            }
            other => panic!("expected SingleEvent AddChar, got {:?}", other),
        }
    }

    #[test]
    fn bulk_change_flushes_on_non_move_event_boundaries() {
        let (tx, rx) = channel::<ChangeEvent>();

        let from1 = bc(0, 0, 'x');
        let to1 = bc(0, 1, 'x');
        let removed = bc(0, 1, 'y');
        let from2 = bc(1, 0, 'z');
        let to2 = bc(1, 1, 'z');

        tx.send(ChangeEvent::MoveChar {
            from: from1,
            to: to1,
        })
        .unwrap();
        tx.send(ChangeEvent::RemoveChar(removed)).unwrap();
        tx.send(ChangeEvent::MoveChar {
            from: from2,
            to: to2,
        })
        .unwrap();

        let events = bulk_change_events(&rx);
        assert_eq!(events.len(), 3);

        match &events[0] {
            BulkedChangeEvent::MultipleEvents(buffered) => {
                assert_eq!(buffered, &vec![(from1, to1)]);
            }
            other => panic!("expected first MultipleEvents, got {:?}", other),
        }

        match &events[1] {
            BulkedChangeEvent::SingleEvent(ChangeEvent::RemoveChar(c)) => {
                assert_eq!(c, &removed);
            }
            other => panic!("expected middle SingleEvent RemoveChar, got {:?}", other),
        }

        match &events[2] {
            BulkedChangeEvent::MultipleEvents(buffered) => {
                assert_eq!(buffered, &vec![(from2, to2)]);
            }
            other => panic!("expected last MultipleEvents, got {:?}", other),
        }
    }
}
