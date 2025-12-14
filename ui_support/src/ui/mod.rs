mod card;
mod ime_input;
mod select_option;
mod selectbox;
mod single_line;
mod single_svg;
mod text_input;
mod textedit;
mod view_element_state;
mod stack_layout;

pub use card::Card;
pub use ime_input::ImeInput;
pub use select_option::SelectOption;
pub use selectbox::SelectBox;
pub use single_line::SingleLine;
pub use single_svg::SingleSvg;
pub use text_input::TextInput;
pub use textedit::TextEdit;
pub use stack_layout::StackLayout;

use font_rasterizer::color_theme::{ColorTheme, ThemedColor};
use text_buffer::caret::CaretType;

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

#[cfg(test)]
mod test {
    use super::split_preedit_string;

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
}
