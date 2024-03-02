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

    pub fn calc_phisical_layout(
        &self,
        max_line_width: usize,
        _line_boundary_prohibited_chars: &LineBoundaryProhibitedChars,
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
            for buffer_char in line.chars.iter() {
                // 物理位置を計算
                let char_width = width_resolver.resolve_width(buffer_char.c);
                // TODO 禁則文字の計算をするならこのあたりでやる
                if buffer_char.col == 0 {
                    // 論理行の行頭では禁則文字を考慮しない
                } else if phisical_col + char_width > max_line_width {
                    phisical_row += 1;
                    phisical_col = 0;
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
            max_width: usize,
            main_caret_pos: PhisicalPosition,
            mark_pos: Option<PhisicalPosition>,
        }
        let cases = vec![
            TestCase {
                input: vec![EditorOperation::InsertString(
                    "ABCDE\nFGHIJ\nKLMNO".to_string(),
                )],
                max_width: 4,
                main_caret_pos: PhisicalPosition { row: 5, col: 1 },
                mark_pos: None,
            },
            TestCase {
                input: vec![EditorOperation::InsertString(
                    "ABCDE\nFGHIJ\nKLMNO".to_string(),
                )],
                max_width: 10,
                main_caret_pos: PhisicalPosition { row: 2, col: 5 },
                mark_pos: None,
            },
            TestCase {
                input: vec![EditorOperation::InsertString("日本の四季折々".to_string())],
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
            layout.chars.iter().for_each(|(c, p)| {
                println!("{:?} {:?}", c, p);
            });
            println!("main_caret :{:?}", layout.main_caret_pos);
            println!("mark       :{:?}", layout.mark_pos);
            assert_eq!(layout.main_caret_pos, case.main_caret_pos);
            assert_eq!(layout.mark_pos, case.mark_pos);
        }
    }
}
