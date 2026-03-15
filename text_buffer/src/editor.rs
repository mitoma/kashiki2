use std::fmt::Display;
use std::sync::Arc;
use std::sync::mpsc::Sender;

use super::action::*;
use super::buffer::*;
use super::caret::*;
use crate::notifier::{ChangeEventNotifier, SharedChangeEventNotifier, shared_notifier};

#[derive(Default)]
struct SelectionState {
    mark: Option<Caret>,
}

impl SelectionState {
    fn mark_caret(&self) -> Option<Caret> {
        self.mark
    }

    fn mark_caret_mut(&mut self) -> &mut Option<Caret> {
        &mut self.mark
    }

    fn mark(&mut self, main_caret: Caret, notifier: &dyn ChangeEventNotifier) {
        if let Some(current_mark) = self.mark {
            notifier.notify(ChangeEvent::RemoveCaret(current_mark));
        }
        self.mark = Some(Caret::new_mark_with_notifier(main_caret.position, notifier));
    }

    fn unmark(&mut self, notifier: &dyn ChangeEventNotifier) {
        if let Some(current_mark) = self.mark.take() {
            notifier.notify(ChangeEvent::RemoveCaret(current_mark));
        }
    }

    fn selection(&self, main_caret: Caret, buffer: &Buffer) -> Vec<BufferChar> {
        let Some(mark) = self.mark else {
            return Vec::new();
        };
        let (from, to) = if main_caret < mark {
            (main_caret.position, mark.position)
        } else {
            (mark.position, main_caret.position)
        };
        if from.is_same_row(&to) {
            let Some(line) = buffer.lines.get(from.row) else {
                return Vec::new();
            };
            let start = from.col.min(line.chars.len());
            let end = to.col.min(line.chars.len());
            return line.chars[start..end].to_vec();
        }

        let mut result = Vec::new();
        if let Some(line) = buffer.lines.get(from.row) {
            let start = from.col.min(line.chars.len());
            result.extend(line.chars[start..].iter().copied());
        }
        for row in from.row.saturating_add(1)..to.row {
            if let Some(line) = buffer.lines.get(row) {
                result.extend(line.chars.iter().copied());
            }
        }
        if let Some(line) = buffer.lines.get(to.row) {
            let end = to.col.min(line.chars.len());
            result.extend(line.chars[..end].iter().copied());
        }
        result
    }

    fn notify_selection_delta(
        pre_selection: &[BufferChar],
        post_selection: &[BufferChar],
        notifier: &dyn ChangeEventNotifier,
    ) {
        pre_selection
            .iter()
            .filter(|c| !post_selection.contains(c))
            .copied()
            .for_each(|c| notifier.notify(ChangeEvent::UnSelectChar(c)));

        post_selection
            .iter()
            .filter(|c| !pre_selection.contains(c))
            .copied()
            .for_each(|c| notifier.notify(ChangeEvent::SelectChar(c)));
    }
}

struct IndentSettings {
    list_indent_patterns: &'static [&'static str],
}

impl Default for IndentSettings {
    fn default() -> Self {
        Self {
            list_indent_patterns: &DEFAULT_LIST_INDENT_PATTERN,
        }
    }
}

pub struct Editor {
    main_caret: Caret,
    selection_state: SelectionState,
    buffer: Buffer,
    undo_list: Vec<ReverseActions>,
    sender: Sender<ChangeEvent>,
    notifier: SharedChangeEventNotifier,
}

impl Editor {
    pub fn new(sender: Sender<ChangeEvent>) -> Self {
        let notifier = shared_notifier(sender.clone());
        Self {
            main_caret: Caret::new_primary_with_notifier([0, 0].into(), notifier.as_ref()),
            selection_state: SelectionState::default(),
            buffer: Buffer::new(sender.clone()),
            undo_list: Vec::new(),
            sender,
            notifier,
        }
    }

    // action を実行する前後で selection が変わった場合に、変更を sender に通知する
    #[inline]
    fn action_with_selection_update(
        &mut self,
        op: &EditorOperation,
        action: impl FnOnce(&mut Self),
    ) {
        let pre_selection = self.selection();
        let should_preclear_selection = op.is_unmark_operation();

        // unmark 対象の操作の場合は action 実行前に選択範囲解除のためのイベントを送信する
        // なぜならアクション実行後にイベントを送信しても
        // BufferChar の座標が変わっていて正しく選択範囲を解除できないため
        if should_preclear_selection {
            pre_selection
                .iter()
                .copied()
                .for_each(|c| self.notifier.notify(ChangeEvent::UnSelectChar(c)));
        }

        action(self);

        let post_selection = self.selection();
        let diff_base = if should_preclear_selection {
            Vec::new()
        } else {
            pre_selection
        };
        if diff_base != post_selection {
            SelectionState::notify_selection_delta(
                &diff_base,
                &post_selection,
                self.notifier.as_ref(),
            );
        }
    }

    pub fn operation(&mut self, op: &EditorOperation) {
        self.action_with_selection_update(op, |itself| {
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
                itself.selection_state.mark_caret_mut(),
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
                self.selection_state.mark_caret_mut(),
                &reverse_action,
                &self.sender,
            );
        }
    }

    pub fn mark(&mut self) {
        self.selection_state
            .mark(self.main_caret, self.notifier.as_ref());
    }

    pub fn unmark(&mut self) {
        self.selection_state.unmark(self.notifier.as_ref());
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
        self.selection_state
            .selection(self.main_caret, &self.buffer)
    }

    /// 箇条書きのような行では折り返す場合のインデント数を計算する
    /// 例えば "  - ABCDEFG" という行があった場合、折り返し時には "    EFG" のようにインデントを入れるため 4 を返す
    /// 引数の line_string は行全体の文字列を表す
    #[inline]
    fn calc_indent(
        line_string: &str,
        width_resolver: Arc<dyn CharWidthResolver>,
        indent_settings: &IndentSettings,
    ) -> usize {
        let mut list_indent_pattern = indent_settings.list_indent_patterns.to_vec();
        // インデントパターンを長い順で評価しないと、長いパターンが使われないケースがある
        // 例えば "- " と "- [ ] " がある場合、"- [ ] " が先に評価されないと "- " がマッチしてしまう
        list_indent_pattern.sort_by(|l, r| l.len().cmp(&r.len()).reverse());
        for pattern in list_indent_pattern {
            if line_string.trim_start().starts_with(pattern) {
                // line_string の何文字目に pattern がマッチするかを取得する
                let Some(space_num) = line_string.find(pattern) else {
                    continue;
                };
                let space_size = line_string[0..space_num]
                    .chars()
                    .map(|c| width_resolver.resolve_width(c))
                    .sum::<usize>();
                let pattern_size = pattern
                    .chars()
                    .map(|c| width_resolver.resolve_width(c))
                    .sum::<usize>();
                return space_size + pattern_size;
            }
        }
        0
    }

    pub fn calc_phisical_layout(
        &self,
        max_line_width: usize,
        line_boundary_prohibited_chars: &LineBoundaryProhibitedChars,
        width_resolver: Arc<dyn CharWidthResolver>,
    ) -> PhisicalLayout {
        self.calc_physical_layout_with_settings(
            max_line_width,
            line_boundary_prohibited_chars,
            width_resolver,
            &IndentSettings::default(),
        )
    }

    fn calc_physical_layout_with_settings(
        &self,
        max_line_width: usize,
        line_boundary_prohibited_chars: &LineBoundaryProhibitedChars,
        width_resolver: Arc<dyn CharWidthResolver>,
        indent_settings: &IndentSettings,
    ) -> PhisicalLayout {
        let mut chars = Vec::new();
        let mut phisical_row = 0;

        let mut main_caret_pos = PhisicalPosition { row: 0, col: 0 };
        let mark_caret = self.selection_state.mark_caret();
        let mut mark_pos = mark_caret.map(|_| PhisicalPosition { row: 0, col: 0 });

        for line in self.buffer.lines.iter() {
            let mut phisical_col = 0;

            // 空行に caret だけ存在するケース
            if line.chars.is_empty() {
                if self.main_caret.position.row == line.row_num {
                    main_caret_pos.row = phisical_row;
                    main_caret_pos.col = phisical_col;
                }
                if let (Some(mark), Some(mark_caret)) = (mark_pos.as_mut(), mark_caret)
                    && mark_caret.position.row == line.row_num
                {
                    mark.row = phisical_row;
                    mark.col = phisical_col;
                }
            }

            // 箇条書きっぽい行では折り返し時にインデントを入れる
            let indent = Self::calc_indent(
                &line.to_line_string(),
                width_resolver.clone(),
                indent_settings,
            );
            for buffer_char in line.chars.iter() {
                // 物理位置を計算
                let char_width = width_resolver.resolve_width(buffer_char.c);

                // 禁則文字の計算
                if buffer_char.position.col == 0 {
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
                if let (Some(mark), Some(mark_caret)) = (mark_pos.as_mut(), mark_caret) {
                    Self::update_caret_position(
                        mark,
                        &mark_caret,
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

    /// caret_pos を更新するヘルパー関数
    #[inline]
    fn update_caret_position(
        caret_pos: &mut PhisicalPosition,
        caret: &Caret,
        buffer_char: &BufferChar,
        phisical_row: usize,
        phisical_col: usize,
        char_width: usize,
    ) {
        let caret = caret.position;
        let buffer_char = buffer_char.position;
        // caret と buffer_char が同じ位置にある場合、caret_pos を更新する
        if caret.row == buffer_char.row {
            if caret.col == buffer_char.col {
                caret_pos.row = phisical_row;
                caret_pos.col = phisical_col;
            } else if caret.col == buffer_char.col + 1 {
                // caret が buffer_char の次の位置にある場合、caret_pos を更新する
                // (行末の場合に位置を計算するためにこの処理が必要になる)
                caret_pos.row = phisical_row;
                caret_pos.col = phisical_col + char_width;
            }
        }
    }
}

// 画面表示の都合の折り返しや禁則文字を考慮した文字列のレイアウトを表す構造体
#[derive(Debug)]
pub struct PhisicalLayout {
    pub chars: Vec<(BufferChar, PhisicalPosition)>,
    pub main_caret_pos: PhisicalPosition,
    pub mark_pos: Option<PhisicalPosition>,
}

impl Display for PhisicalLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
        write!(f, "{}", result)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub struct PhisicalPosition {
    pub row: usize,
    pub col: usize,
}

const DEFAULT_LIST_INDENT_PATTERN: [&str; 17] = [
    "* ", "* [ ] ", "* [x] ", "- ", "- [ ] ", "- [x] ", "> ", "・",
    // TODO 数字の箇条書きをもっとちゃんとサポートしたいが、現状は 1. ～ 9. まで雑に対応
    "1. ", "2. ", "3. ", "4. ", "5. ", "6. ", "7. ", "8. ", "9. ",
];

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
    SelectChar(BufferChar),
    UnSelectChar(BufferChar),
    AddCaret(Caret),
    MoveCaret { from: Caret, to: Caret },
    RemoveCaret(Caret),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::Receiver;

    struct TestWidthResolver;

    impl CharWidthResolver for TestWidthResolver {
        fn resolve_width(&self, c: char) -> usize {
            // テスト用なので雑に判定
            if c.is_ascii() { 1 } else { 2 }
        }
    }

    fn collect_events(receiver: &Receiver<ChangeEvent>) -> Vec<ChangeEvent> {
        receiver.try_iter().collect()
    }

    #[test]
    fn selection_events_follow_caret_diff() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut editor = Editor::new(sender);
        editor.operation(&EditorOperation::InsertString("ABCDE".to_string()));
        editor.operation(&EditorOperation::BufferHead);
        editor.operation(&EditorOperation::Forward);
        editor.operation(&EditorOperation::Mark);
        let _ = collect_events(&receiver);

        editor.operation(&EditorOperation::Forward);
        assert_eq!(
            collect_events(&receiver),
            vec![
                ChangeEvent::MoveCaret {
                    from: Caret::new_without_event([0, 1].into(), CaretType::Primary),
                    to: Caret::new_without_event([0, 2].into(), CaretType::Primary),
                },
                ChangeEvent::SelectChar(BufferChar {
                    position: [0, 1].into(),
                    c: 'B',
                }),
            ]
        );

        editor.operation(&EditorOperation::Forward);
        assert_eq!(
            collect_events(&receiver),
            vec![
                ChangeEvent::MoveCaret {
                    from: Caret::new_without_event([0, 2].into(), CaretType::Primary),
                    to: Caret::new_without_event([0, 3].into(), CaretType::Primary),
                },
                ChangeEvent::SelectChar(BufferChar {
                    position: [0, 2].into(),
                    c: 'C',
                }),
            ]
        );

        editor.operation(&EditorOperation::Back);
        assert_eq!(
            collect_events(&receiver),
            vec![
                ChangeEvent::MoveCaret {
                    from: Caret::new_without_event([0, 3].into(), CaretType::Primary),
                    to: Caret::new_without_event([0, 2].into(), CaretType::Primary),
                },
                ChangeEvent::UnSelectChar(BufferChar {
                    position: [0, 2].into(),
                    c: 'C',
                }),
            ]
        );
    }

    #[test]
    fn unmark_clears_selection_before_removing_mark() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut editor = Editor::new(sender);
        editor.operation(&EditorOperation::InsertString("ABCDE".to_string()));
        editor.operation(&EditorOperation::BufferHead);
        editor.operation(&EditorOperation::Forward);
        editor.operation(&EditorOperation::Mark);
        editor.operation(&EditorOperation::Forward);
        editor.operation(&EditorOperation::Forward);
        let _ = collect_events(&receiver);

        editor.operation(&EditorOperation::UnMark);
        assert_eq!(
            collect_events(&receiver),
            vec![
                ChangeEvent::UnSelectChar(BufferChar {
                    position: [0, 1].into(),
                    c: 'B',
                }),
                ChangeEvent::UnSelectChar(BufferChar {
                    position: [0, 2].into(),
                    c: 'C',
                }),
                ChangeEvent::RemoveCaret(Caret::new_without_event([0, 1].into(), CaretType::Mark,)),
            ]
        );
    }

    #[test]
    fn undo_keeps_public_event_order() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut editor = Editor::new(sender);
        editor.operation(&EditorOperation::InsertString("ab".to_string()));
        let _ = collect_events(&receiver);

        editor.operation(&EditorOperation::Undo);
        assert_eq!(editor.to_buffer_string(), "");
        assert_eq!(
            collect_events(&receiver),
            vec![
                ChangeEvent::MoveCaret {
                    from: Caret::new_without_event([0, 2].into(), CaretType::Primary),
                    to: Caret::new_without_event([0, 1].into(), CaretType::Primary),
                },
                ChangeEvent::RemoveChar(BufferChar {
                    position: [0, 1].into(),
                    c: 'b',
                }),
                ChangeEvent::MoveCaret {
                    from: Caret::new_without_event([0, 1].into(), CaretType::Primary),
                    to: Caret::new_without_event([0, 0].into(), CaretType::Primary),
                },
                ChangeEvent::RemoveChar(BufferChar {
                    position: [0, 0].into(),
                    c: 'a',
                }),
            ]
        );
    }

    #[test]
    fn disconnected_receiver_does_not_panic() {
        let (sender, receiver) = std::sync::mpsc::channel();
        drop(receiver);

        let mut editor = Editor::new(sender);
        editor.operation(&EditorOperation::InsertString("abc".to_string()));
        editor.operation(&EditorOperation::BufferHead);
        editor.operation(&EditorOperation::Mark);
        editor.operation(&EditorOperation::Forward);
        editor.operation(&EditorOperation::UnMark);

        let layout = editor.calc_phisical_layout(
            10,
            &LineBoundaryProhibitedChars::default(),
            Arc::new(TestWidthResolver),
        );
        assert_eq!(editor.to_buffer_string(), "abc");
        assert_eq!(editor.buffer_chars()[0].len(), 3);
        assert_eq!(layout.main_caret_pos, PhisicalPosition { row: 0, col: 1 });
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
        for case in cases.iter() {
            let (sender, receiver) = std::sync::mpsc::channel();
            let mut editor = Editor::new(sender.clone());
            case.input.iter().for_each(|op| editor.operation(op));
            let _ = receiver.try_iter().collect::<Vec<_>>();

            let layout = editor.calc_phisical_layout(
                case.max_width,
                &LineBoundaryProhibitedChars::new(vec![], vec![]),
                Arc::new(TestWidthResolver),
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
            );
            assert_eq!(layout.to_string(), case.output);
        }
    }
}
