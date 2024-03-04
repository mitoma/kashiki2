use std::sync::mpsc::Sender;

use super::action::*;
use super::buffer::*;
use super::caret::*;

pub struct Editor {
    main_caret: Caret,
    mark: Option<Caret>,
    buffer: Buffer,
    undo_list: Vec<ReverseActions>,
    sender: Sender<ChangeEvent>,
}

impl Editor {
    pub fn new(sender: Sender<ChangeEvent>) -> Self {
        Self {
            main_caret: Caret::new(0, 0, &sender),
            mark: Option::None,
            buffer: Buffer::new(sender.clone()),
            undo_list: Vec::new(),
            sender,
        }
    }

    pub fn operation(&mut self, op: &EditorOperation) {
        if EditorOperation::Undo == *op {
            self.undo();
            return;
        }
        if EditorOperation::Mark == *op {
            self.mark();
            return;
        }
        let reverse_actions = BufferApplyer::apply_action(
            &mut self.buffer,
            &mut self.main_caret,
            &mut self.mark,
            op,
            &self.sender,
        );
        self.undo_list.push(reverse_actions);
    }

    fn undo(&mut self) {
        if let Some(reverse_action) = self.undo_list.pop() {
            BufferApplyer::apply_reserve_actions(
                &mut self.buffer,
                &mut self.main_caret,
                &mut self.mark,
                &reverse_action,
                &self.sender,
            );
        }
    }

    pub fn mark(&mut self) {
        if let Some(current_mark) = self.mark {
            self.sender
                .send(ChangeEvent::RemoveCaret(current_mark))
                .unwrap();
        }
        self.mark = Some(Caret::new(
            self.main_caret.row,
            self.main_caret.col,
            &self.sender,
        ));
    }

    pub fn to_buffer_string(&self) -> String {
        self.buffer.to_buffer_string()
    }

    #[inline]
    fn calc_indent(line_string: &str, width_resolver: &dyn CharWidthResolver) -> usize {
        let mut list_indent_pattern: Vec<&str> =
            vec!["* ", "* [ ] ", "* [x] ", "- ", "- [ ] ", "- [x] ", "・"];
        list_indent_pattern.sort_by(|l, r| l.len().cmp(&r.len()).reverse());
        for pattern in list_indent_pattern {
            if line_string.trim_start().starts_with(pattern) {
                // line_string の何文字目に pattern がマッチするかを取得する
                return line_string.find(pattern).unwrap()
                    + pattern
                        .chars()
                        .map(|c| width_resolver.resolve_width(c))
                        .sum::<usize>();
            }
        }
        0
    }

    pub fn calc_phisical_layout(
        &self,
        max_line_width: usize,
        line_boundary_prohibited_chars: &LineBoundaryProhibitedChars,
        width_resolver: &dyn CharWidthResolver,
    ) -> PhisicalLayout {
        let mut chars = Vec::new();
        let mut phisical_row = 0;

        let mut main_caret_pos = PhisicalPosition { row: 0, col: 0 };
        let mut mark_pos = self.mark.map(|_| PhisicalPosition { row: 0, col: 0 });

        for line in self.buffer.lines.iter() {
            let mut phisical_col = 0;

            // 空行に caret だけ存在するケース
            if line.chars.is_empty() {
                if self.main_caret.row == line.row_num {
                    main_caret_pos.row = phisical_row;
                    main_caret_pos.col = phisical_col;
                }
                if let Some(mark) = mark_pos.as_mut() {
                    if self.mark.unwrap().row == line.row_num {
                        mark.row = phisical_row;
                        mark.col = phisical_col;
                    }
                }
            }

            // 箇条書きっぽい行では折り返し時にインデントを入れる
            let indent = Self::calc_indent(&line.to_line_string(), width_resolver);
            for buffer_char in line.chars.iter() {
                // 物理位置を計算
                let char_width = width_resolver.resolve_width(buffer_char.c);

                // 禁則文字の計算
                if buffer_char.col == 0 {
                    // 論理行の行頭では禁則文字を考慮しない
                } else if phisical_col + char_width >= max_line_width
                    && line_boundary_prohibited_chars.end.contains(&buffer_char.c)
                {
                    // 行末禁則文字の場合は max_line_width に達する前に改行する
                    phisical_row += 1;
                    phisical_col = indent;
                } else if phisical_col + char_width > max_line_width {
                    if line_boundary_prohibited_chars
                        .start
                        .contains(&buffer_char.c)
                        && max_line_width >= phisical_col
                    {
                        // 行頭禁則文字の場合は max_line_width を超えていても 1 文字だけ改行しない
                    } else {
                        phisical_row += 1;
                        phisical_col = indent;
                    }
                }

                // char の位置を確定
                let phisical_position = PhisicalPosition {
                    row: phisical_row,
                    col: phisical_col,
                };
                chars.push((*buffer_char, phisical_position));

                // キャレットの位置を確定
                Self::update_caret_position(
                    &mut main_caret_pos,
                    &self.main_caret,
                    buffer_char,
                    phisical_row,
                    phisical_col,
                    char_width,
                );
                if let Some(mark) = mark_pos.as_mut() {
                    Self::update_caret_position(
                        mark,
                        &self.mark.unwrap(),
                        buffer_char,
                        phisical_row,
                        phisical_col,
                        char_width,
                    );
                }

                phisical_col += char_width;
            }
            phisical_row += 1;
        }
        PhisicalLayout {
            chars,
            main_caret_pos,
            mark_pos,
        }
    }

    #[inline]
    fn update_caret_position(
        caret_pos: &mut PhisicalPosition,
        caret: &Caret,
        buffer_char: &BufferChar,
        phisical_row: usize,
        phisical_col: usize,
        char_width: usize,
    ) {
        if caret.row == buffer_char.row {
            if caret.col == buffer_char.col {
                caret_pos.row = phisical_row;
                caret_pos.col = phisical_col;
            } else if caret.col == buffer_char.col + 1 {
                caret_pos.row = phisical_row;
                caret_pos.col = phisical_col + char_width;
            }
        }
    }
}

#[derive(Debug)]
pub struct PhisicalLayout {
    pub chars: Vec<(BufferChar, PhisicalPosition)>,
    pub main_caret_pos: PhisicalPosition,
    pub mark_pos: Option<PhisicalPosition>,
}

impl ToString for PhisicalLayout {
    fn to_string(&self) -> String {
        let mut current_row = 0;
        let mut result = String::new();
        for (c, position) in self.chars.iter() {
            while current_row != position.row {
                result.push('\n');
                result.push_str(&" ".repeat(position.col));
                current_row += 1;
            }
            result.push(c.c);
        }
        result
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub struct PhisicalPosition {
    pub row: usize,
    pub col: usize,
}

/// 禁則文字の定義を持つ enum
pub struct LineBoundaryProhibitedChars {
    pub start: Vec<char>,
    pub end: Vec<char>,
}

impl LineBoundaryProhibitedChars {
    pub fn new(start: Vec<char>, end: Vec<char>) -> Self {
        Self { start, end }
    }
}

const DEFAULT_STARTS: &str =
    ",.!?;:)]}”’）〉》〕〗〙〛｝〉》〕〗〙〛｝」』】、。！？；：-ー…～〃々ゝゞヽヾ";
const DEFAULT_ENDS: &str = "([{“‘（〈《〔〖〘〚｛〈《〔〖〘〚｛「『【";

impl Default for LineBoundaryProhibitedChars {
    fn default() -> Self {
        Self {
            start: DEFAULT_STARTS.chars().collect(),
            end: DEFAULT_ENDS.chars().collect(),
        }
    }
}

/// 文字の幅を解決する trait
pub trait CharWidthResolver {
    fn resolve_width(&self, char: char) -> usize;
}

#[derive(Debug, PartialEq, Eq)]
pub enum ChangeEvent {
    AddChar(BufferChar),
    MoveChar { from: BufferChar, to: BufferChar },
    RemoveChar(BufferChar),
    AddCaret(Caret),
    MoveCaret { from: Caret, to: Caret },
    RemoveCaret(Caret),
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestWidthResolver;

    impl CharWidthResolver for TestWidthResolver {
        fn resolve_width(&self, c: char) -> usize {
            // テスト用なので雑に判定
            if c.is_ascii() {
                1
            } else {
                2
            }
        }
    }

    #[test]
    fn test_calc_phisical_layout() {
        struct TestCase {
            input: Vec<EditorOperation>,
            output: String,
            max_width: usize,
            main_caret_pos: PhisicalPosition,
            mark_pos: Option<PhisicalPosition>,
        }
        let cases = vec![
            TestCase {
                input: vec![EditorOperation::InsertString(
                    "ABCDE\nFGHIJ\nKLMNO".to_string(),
                )],
                output: "ABCD\nE\nFGHI\nJ\nKLMN\nO".to_string(),
                max_width: 4,
                main_caret_pos: PhisicalPosition { row: 5, col: 1 },
                mark_pos: None,
            },
            TestCase {
                input: vec![EditorOperation::InsertString(
                    "ABCDE\nFGHIJ\nKLMNO".to_string(),
                )],
                output: "ABCDE\nFGHIJ\nKLMNO".to_string(),
                max_width: 10,
                main_caret_pos: PhisicalPosition { row: 2, col: 5 },
                mark_pos: None,
            },
            TestCase {
                input: vec![EditorOperation::InsertString("日本の四季折々".to_string())],
                output: "日本の四季\n折々".to_string(),
                max_width: 10,
                main_caret_pos: PhisicalPosition { row: 1, col: 4 },
                mark_pos: None,
            },
            TestCase {
                input: vec![
                    EditorOperation::InsertString("\n\n日本の四季折々".to_string()),
                    EditorOperation::BufferHead,
                    EditorOperation::Forward,
                ],
                output: "\n\n日本の四季\n折々".to_string(),
                max_width: 10,
                main_caret_pos: PhisicalPosition { row: 1, col: 0 },
                mark_pos: None,
            },
            TestCase {
                input: vec![
                    EditorOperation::InsertString("ABCDEFGHIJK".to_string()),
                    EditorOperation::BufferHead,
                    EditorOperation::Forward,
                    EditorOperation::Mark,
                    EditorOperation::Forward,
                    EditorOperation::Forward,
                ],
                output: "ABC\nDEF\nGHI\nJK".to_string(),
                max_width: 3,
                main_caret_pos: PhisicalPosition { row: 1, col: 0 },
                mark_pos: Some(PhisicalPosition { row: 0, col: 1 }),
            },
        ];
        for case in cases.iter() {
            let (sender, receiver) = std::sync::mpsc::channel();
            let mut editor = Editor::new(sender.clone());
            case.input.iter().for_each(|op| editor.operation(op));
            let _ = receiver.try_iter().collect::<Vec<_>>();

            let layout = editor.calc_phisical_layout(
                case.max_width,
                &LineBoundaryProhibitedChars::new(vec![], vec![]),
                &TestWidthResolver,
            );
            assert_eq!(layout.to_string(), case.output);
            assert_eq!(layout.main_caret_pos, case.main_caret_pos);
            assert_eq!(layout.mark_pos, case.mark_pos);
        }
    }

    // 禁則文字の実装のテスト
    #[test]
    fn test_line_boundary_prohibited_chars() {
        struct TestCase {
            input: Vec<EditorOperation>,
            output: String,
            prohibited_chars: LineBoundaryProhibitedChars,
            max_width: usize,
        }
        let cases = vec![
            TestCase {
                input: vec![EditorOperation::InsertString(
                    "ABCDE\nFGHIJ\nKLMNO".to_string(),
                )],
                output: "ABCD\nE\nFGHI\nJ\nKLMN\nO".to_string(),
                prohibited_chars: LineBoundaryProhibitedChars::default(),
                max_width: 4,
            },
            // 行頭禁則文字のテストケース
            TestCase {
                input: vec![EditorOperation::InsertString(
                    "こんにちは。山田です。".to_string(),
                )],
                output: "こんにちは。\n山田です。".to_string(),
                prohibited_chars: LineBoundaryProhibitedChars::default(),
                max_width: 10,
            },
            TestCase {
                input: vec![EditorOperation::InsertString(
                    "Hello, World! And you.".to_string(),
                )],
                output: "Hello, World!\n And you.".to_string(),
                prohibited_chars: LineBoundaryProhibitedChars::default(),
                max_width: 12,
            },
            // 行末禁則文字のテストケース
            TestCase {
                input: vec![EditorOperation::InsertString(
                    "あなたが「本物」ですね。".to_string(),
                )],
                output: "あなたが\n「本物」で\nすね。".to_string(),
                prohibited_chars: LineBoundaryProhibitedChars::default(),
                max_width: 10,
            },
            TestCase {
                input: vec![EditorOperation::InsertString(
                    "Power is [chikara]".to_string(),
                )],
                output: "Power is \n[chikara]".to_string(),
                prohibited_chars: LineBoundaryProhibitedChars::default(),
                max_width: 10,
            },
        ];
        for case in cases.iter() {
            let (sender, receiver) = std::sync::mpsc::channel();
            let mut editor = Editor::new(sender.clone());
            case.input.iter().for_each(|op| editor.operation(op));
            let _ = receiver.try_iter().collect::<Vec<_>>();

            let layout = editor.calc_phisical_layout(
                case.max_width,
                &case.prohibited_chars,
                &TestWidthResolver,
            );
            assert_eq!(layout.to_string(), case.output);
        }
    }
    #[test]
    fn test_indent() {
        struct TestCase {
            input: Vec<EditorOperation>,
            output: String,
            prohibited_chars: LineBoundaryProhibitedChars,
            max_width: usize,
        }
        let cases = vec![
            TestCase {
                input: vec![EditorOperation::InsertString(
                    "- [ ] abcdefghijklmn".to_string(),
                )],
                output: "- [ ] abcd\n      efgh\n      ijkl\n      mn".to_string(),
                prohibited_chars: LineBoundaryProhibitedChars::default(),
                max_width: 10,
            },
            TestCase {
                input: vec![EditorOperation::InsertString(
                    "  - スーパーマンはどこにいる？".to_string(),
                )],
                output: "  - スーパー\n    マンは\n    どこに\n    いる？".to_string(),
                prohibited_chars: LineBoundaryProhibitedChars::default(),
                max_width: 10,
            },
        ];
        for case in cases.iter() {
            let (sender, receiver) = std::sync::mpsc::channel();
            let mut editor = Editor::new(sender.clone());
            case.input.iter().for_each(|op| editor.operation(op));
            let _ = receiver.try_iter().collect::<Vec<_>>();

            let layout = editor.calc_phisical_layout(
                case.max_width,
                &case.prohibited_chars,
                &TestWidthResolver,
            );
            assert_eq!(layout.to_string(), case.output);
        }
    }
}
