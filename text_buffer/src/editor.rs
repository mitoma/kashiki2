use std::sync::Arc;
use std::sync::mpsc::Sender;

use super::action::*;
use super::buffer::*;
use super::caret::*;
use super::phisical_layout_calcurator::PhisicalLayoutCalcurator;

pub use super::phisical_layout_calcurator::{
    CharWidthResolver, LineBoundaryProhibitedChars, PhisicalLayout, PhisicalPosition,
};

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
            main_caret: Caret::new([0, 0].into(), &sender),
            mark: Option::None,
            buffer: Buffer::new(sender.clone()),
            undo_list: Vec::new(),
            sender,
        }
    }

    // action を実行する前後で selection が変わった場合に、変更を sender に通知する
    #[inline]
    fn action_width_selection_update(
        &mut self,
        op: &EditorOperation,
        action: impl FnOnce(&mut Self),
    ) {
        let pre_selection = self.selection();

        // unmark 対象の操作の場合は action 実行前に選択範囲解除のためのイベントを送信する
        // なぜならアクション実行後にイベントを送信しても
        // BufferChar の座標が変わっていて正しく選択範囲を解除できないため
        if op.is_unmark_operation() {
            pre_selection.iter().cloned().for_each(|c| {
                self.sender.send(ChangeEvent::UnSelectChar(c)).unwrap();
            });
        }

        action(self);

        let post_selection = self.selection();
        if pre_selection != post_selection {
            let leave_selections = pre_selection
                .iter()
                .filter(|c| !post_selection.contains(c))
                .cloned()
                .collect::<Vec<_>>();
            leave_selections.iter().for_each(|c| {
                self.sender.send(ChangeEvent::UnSelectChar(*c)).unwrap();
            });

            let enter_selections = post_selection
                .iter()
                .filter(|c| !pre_selection.contains(c))
                .cloned()
                .collect::<Vec<_>>();
            enter_selections.iter().for_each(|c| {
                self.sender.send(ChangeEvent::SelectChar(*c)).unwrap();
            });
        }
    }

    pub fn operation(&mut self, op: &EditorOperation) {
        self.action_width_selection_update(op, |itself| {
            match op {
                EditorOperation::Undo => {
                    itself.undo();
                    return;
                }
                EditorOperation::Mark => {
                    itself.mark();
                    return;
                }
                EditorOperation::UnMark => {
                    itself.unmark();
                    return;
                }
                _ => (),
            }
            let reverse_actions = BufferApplyer::apply_action(
                &mut itself.buffer,
                &mut itself.main_caret,
                &mut itself.mark,
                op,
                &itself.sender,
            );
            itself.undo_list.push(reverse_actions);

            if op.is_unmark_operation() {
                itself.unmark();
            }
        });
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
        self.mark = Some(Caret::new_mark(self.main_caret.position, &self.sender));
    }

    pub fn unmark(&mut self) {
        if let Some(current_mark) = self.mark {
            self.sender
                .send(ChangeEvent::RemoveCaret(current_mark))
                .unwrap();
            self.mark = None;
        }
    }

    pub fn to_buffer_string(&self) -> String {
        self.buffer.to_buffer_string()
    }

    pub fn buffer_chars(&self) -> Vec<Vec<BufferChar>> {
        self.buffer
            .lines
            .iter()
            .map(|line| line.chars.clone())
            .collect()
    }

    fn selection(&self) -> Vec<BufferChar> {
        let Some(mark) = self.mark else {
            return Vec::new();
        };
        let (from, to) = if self.main_caret < mark {
            (self.main_caret.position, mark.position)
        } else {
            (mark.position, self.main_caret.position)
        };
        if from.is_same_row(&to) {
            self.buffer.lines[from.row].chars[from.col..to.col].to_vec()
        } else {
            let mut result = Vec::new();
            result.extend(self.buffer.lines[from.row].chars[from.col..].iter());
            for row in from.row + 1..to.row {
                result.extend(self.buffer.lines[row].chars.iter());
            }
            result.extend(self.buffer.lines[to.row].chars[..to.col].iter());
            result
        }
    }

    pub fn calc_phisical_layout(
        &self,
        max_line_width: usize,
        line_boundary_prohibited_chars: &LineBoundaryProhibitedChars,
        width_resolver: Arc<dyn CharWidthResolver>,
        preedit_string: Option<String>,
    ) -> PhisicalLayout {
        PhisicalLayoutCalcurator::new(
            &self.buffer,
            &self.main_caret,
            self.mark,
            max_line_width,
            line_boundary_prohibited_chars,
            width_resolver,
            preedit_string,
        )
        .calc()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ChangeEvent {
    AddChar(BufferChar),
    MoveChar { from: BufferChar, to: BufferChar },
    RemoveChar(BufferChar),
    SelectChar(BufferChar),
    UnSelectChar(BufferChar),
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
            if c.is_ascii() { 1 } else { 2 }
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
        let cases = [
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
        for (idx, case) in cases.iter().enumerate() {
            let (sender, receiver) = std::sync::mpsc::channel();
            let mut editor = Editor::new(sender.clone());
            case.input.iter().for_each(|op| editor.operation(op));
            let _ = receiver.try_iter().collect::<Vec<_>>();

            let layout = editor.calc_phisical_layout(
                case.max_width,
                &LineBoundaryProhibitedChars::new(vec![], vec![]),
                Arc::new(TestWidthResolver),
                None,
            );
            assert_eq!(layout.to_string(), case.output, "case index: {}", idx);
            assert_eq!(
                layout.main_caret_pos, case.main_caret_pos,
                "case index: {}",
                idx
            );
            assert_eq!(layout.mark_pos, case.mark_pos, "case index: {}", idx);
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
        let cases = [
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
                Arc::new(TestWidthResolver),
                None,
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
        let cases = [
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
            TestCase {
                input: vec![EditorOperation::InsertString(
                    "　- 全角文字、ゆるせん！".to_string(),
                )],
                output: "　- 全角文\n    字、ゆ\n    るせん！".to_string(),
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
                Arc::new(TestWidthResolver),
                None,
            );
            assert_eq!(layout.to_string(), case.output);
        }
    }

    #[test]
    fn test_preedit() {
        struct TestCase {
            input: Vec<EditorOperation>,
            preedit_string: String,
            output: String,
            prohibited_chars: LineBoundaryProhibitedChars,
            max_width: usize,
        }
        let cases = [
            TestCase {
                input: vec![
                    EditorOperation::InsertString("こんにちはさん".to_string()),
                    EditorOperation::Back,
                    EditorOperation::Back,
                ],
                preedit_string: "山田太郎".to_string(),
                output: "こんにちはさん".to_string(),
                prohibited_chars: LineBoundaryProhibitedChars::default(),
                max_width: 100,
            },
            // 折り返しが起きるテストケース：preedit が max_width を超える
            TestCase {
                input: vec![EditorOperation::InsertString("inline".to_string())],
                preedit_string: "ダダダダダダ".to_string(), // 12文字 = 24 の幅
                output: "inline".to_string(),
                prohibited_chars: LineBoundaryProhibitedChars::default(),
                max_width: 10, // 折り返しが起きる
            },
            // 折り返し後に行頭禁則文字が来るケース
            TestCase {
                input: vec![EditorOperation::InsertString("text".to_string())],
                preedit_string: "ダダ。ダ".to_string(), // 「。」が行頭禁則文字
                output: "text".to_string(),
                prohibited_chars: LineBoundaryProhibitedChars::default(),
                max_width: 8, // "text" = 4 + "ダ" = 2 で 6、次の "ダ" で 8、次の "。" で 10 > 8 なので行頭禁則関連
            },
            // 改行直後に行頭禁則文字が来るケース（真の問題）
            TestCase {
                input: vec![EditorOperation::InsertString("x".to_string())],
                preedit_string: "ダダ。。".to_string(), // 改行後の「。」で is_line_head ミス問題を再現
                output: "x".to_string(),
                prohibited_chars: LineBoundaryProhibitedChars::default(),
                max_width: 6, // "x" = 1 + "ダ" = 2 で 3、"ダ" で 5、"。" で 7 > 6 → 改行すべき
                              // 改行後 col=0 の新行で「。」が来る
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
                Arc::new(TestWidthResolver),
                Some(case.preedit_string.clone()),
            );
            println!(
                "\n=== Test case: max_width={}, preedit={} ===",
                case.max_width, case.preedit_string
            );
            println!("preedit_chars layout:");
            for (i, (bc, pos)) in layout.preedit_chars.iter().enumerate() {
                println!(
                    "  [{}] '{}' -> physical row={}, col={}",
                    i, bc.c, pos.row, pos.col
                );
            }
            println!("layout: {:#?}", layout);
            assert_eq!(layout.to_string(), case.output);
        }
    }
}
