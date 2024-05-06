use log::info;
use stroke_parser::Action;
use text_buffer::action::EditorOperation;

use crate::{
    color_theme::ThemedColor,
    context::{CharEasings, StateContext, TextContext},
    instances::GlyphInstances,
    layout_engine::Model,
    ui::textedit::TextEditOperation,
};

use super::textedit::TextEdit;

pub struct ImeInput {
    text_edit: TextEdit,
}

impl Default for ImeInput {
    fn default() -> Self {
        Self::new()
    }
}

impl ImeInput {
    pub fn new() -> Self {
        let config = TextContext {
            char_easings: CharEasings::ignore_camera(),
            max_col: 10,
            hyde_caret: true,
            ..Default::default()
        };
        let mut text_edit = TextEdit::default();
        text_edit.set_config(config);
        text_edit.set_world_scale([0.1, 0.1]);
        text_edit.set_position((5.0, -5.0, 0.0).into());

        Self { text_edit }
    }

    pub fn apply_ime_event(&mut self, action: &Action) -> bool {
        match action {
            Action::ImePreedit(value, position) => {
                self.text_edit.editor_operation(&EditorOperation::Mark);
                self.text_edit
                    .editor_operation(&EditorOperation::BufferHead);
                self.text_edit
                    .editor_operation(&EditorOperation::Cut(|_| {}));
                match position {
                    Some((start, end)) if start != end => {
                        info!("start:{start}, end:{end}");
                        let (first, center, last) =
                            split_preedit_string(value.clone(), *start, *end);
                        let left_separator_len = first.chars().count();
                        // 左のセパレーターの文字数を考慮して + 1 している
                        let right_separator_len = left_separator_len + center.chars().count() + 1;
                        let preedit_str = format!("{}[{}]{}", first, center, last);
                        self.text_edit
                            .editor_operation(&EditorOperation::InsertString(preedit_str));
                        self.text_edit
                            .text_edit_operation(TextEditOperation::SetThemedColor(
                                [0, left_separator_len].into()..[0, left_separator_len + 1].into(),
                                ThemedColor::TextEmphasized,
                            ));
                        self.text_edit
                            .text_edit_operation(TextEditOperation::SetThemedColor(
                                [0, right_separator_len].into()
                                    ..[0, right_separator_len + 1].into(),
                                ThemedColor::TextComment,
                            ));
                    }
                    _ => {
                        self.text_edit
                            .editor_operation(&EditorOperation::InsertString(value.clone()));
                    }
                };
                false
            }
            Action::ImeInput(_) => {
                self.text_edit.editor_operation(&EditorOperation::Mark);
                self.text_edit
                    .editor_operation(&EditorOperation::BufferHead);
                self.text_edit
                    .editor_operation(&EditorOperation::Cut(|_| {}));
                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self, context: &StateContext) {
        self.text_edit.update(context)
    }

    pub fn get_instances(&self) -> Vec<&GlyphInstances> {
        self.text_edit.glyph_instances()
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
