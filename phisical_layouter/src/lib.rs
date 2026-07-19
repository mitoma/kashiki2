use icu_segmenter::LineSegmenter;
use icu_segmenter::options::LineBreakOptions;
use std::fmt::Display;
use std::sync::Arc;

use text_buffer::buffer::{BufferChar, CellPosition};
use text_buffer::caret::Caret;
use text_buffer::editor::Editor;

// 画面表示の都合の折り返しや禁則文字を考慮した文字列のレイアウトを表す構造体
#[derive(Debug)]
pub struct PhysicalLayout {
    pub chars: Vec<(BufferChar, PhysicalPosition)>,
    pub preedit_chars: Vec<(BufferChar, PhysicalPosition)>,
    pub main_caret_pos: PhysicalPosition,
    pub mark_pos: Option<PhysicalPosition>,
}

impl Display for PhysicalLayout {
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
pub struct PhysicalPosition {
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

#[allow(clippy::too_many_arguments)]
pub fn calc_phisical_layout(
    editor: &Editor,
    max_line_width: usize,
    line_boundary_prohibited_chars: &LineBoundaryProhibitedChars,
    width_resolver: Arc<dyn CharWidthResolver>,
    preedit_string: Option<String>,
) -> PhysicalLayout {
    let lines = editor.buffer_chars();
    let main_caret = editor.main_caret();
    let mark = editor.mark_caret();
    PhysicalLayoutCalculator::new(
        &lines,
        main_caret,
        mark,
        max_line_width,
        line_boundary_prohibited_chars,
        width_resolver,
        preedit_string,
    )
    .calc()
}

struct PhysicalLayoutCalculator<'a> {
    lines: &'a [Vec<BufferChar>],
    main_caret: Caret,
    mark: Option<Caret>,
    max_line_width: usize,
    line_boundary_prohibited_chars: &'a LineBoundaryProhibitedChars,
    width_resolver: Arc<dyn CharWidthResolver>,
    preedit_string: Option<String>,
}

impl<'a> PhysicalLayoutCalculator<'a> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        lines: &'a [Vec<BufferChar>],
        main_caret: Caret,
        mark: Option<Caret>,
        max_line_width: usize,
        line_boundary_prohibited_chars: &'a LineBoundaryProhibitedChars,
        width_resolver: Arc<dyn CharWidthResolver>,
        preedit_string: Option<String>,
    ) -> Self {
        Self {
            lines,
            main_caret,
            mark,
            max_line_width,
            line_boundary_prohibited_chars,
            width_resolver,
            preedit_string,
        }
    }

    fn calc(&self) -> PhysicalLayout {
        let mut state = LayoutState::new(self.mark);
        let preedit_opt = self.preedit_string.as_deref();

        for (row_num, line_chars) in self.lines.iter().enumerate() {
            state.phisical_col = 0;
            state.current_row_char_start_index = state.chars.len();
            state.last_break_candidate = None;
            state.is_soft_wrapped_row = false;
            let is_caret_row = self.main_caret.position.row == row_num;
            let line_string: String = line_chars.iter().map(|c| c.c).collect();
            let break_before_chars = self.collect_break_before_chars(&line_string);

            if line_chars.is_empty() {
                self.handle_empty_line(&mut state, row_num, is_caret_row, preedit_opt);
            }

            let indent = self.calc_indent(&line_string);

            for buffer_char in line_chars {
                self.try_insert_preedit_before_char(
                    &mut state,
                    buffer_char,
                    row_num,
                    indent,
                    is_caret_row,
                    preedit_opt,
                );

                let can_break_before = break_before_chars
                    .get(buffer_char.position.col)
                    .copied()
                    .unwrap_or(false);
                self.push_buffer_char(&mut state, buffer_char, indent, can_break_before);
            }

            self.try_insert_preedit_at_line_end(
                &mut state,
                row_num,
                line_chars.len(),
                indent,
                is_caret_row,
                preedit_opt,
            );

            state.phisical_row += 1;
        }

        state.into_layout()
    }

    fn handle_empty_line(
        &self,
        state: &mut LayoutState,
        line_row_num: usize,
        is_caret_row: bool,
        preedit_opt: Option<&str>,
    ) {
        if is_caret_row {
            state.main_caret_pos.row = state.phisical_row;
            state.main_caret_pos.col = state.phisical_col;
        }
        if let Some(mark) = state.mark_pos.as_mut()
            && self
                .mark
                .is_some_and(|caret| caret.position.row == line_row_num)
        {
            mark.row = state.phisical_row;
            mark.col = state.phisical_col;
        }

        if is_caret_row
            && !state.preedit_injected
            && let Some(preedit) = preedit_opt
        {
            self.insert_preedit_chars(
                preedit,
                line_row_num,
                self.main_caret.position.col,
                state,
                0,
            );
            state.main_caret_pos.row = state.phisical_row;
            state.main_caret_pos.col = state.phisical_col;
            state.preedit_injected = true;
        }
    }

    fn try_insert_preedit_before_char(
        &self,
        state: &mut LayoutState,
        buffer_char: &BufferChar,
        line_row_num: usize,
        indent: usize,
        is_caret_row: bool,
        preedit_opt: Option<&str>,
    ) {
        if is_caret_row
            && !state.preedit_injected
            && buffer_char.position.col == self.main_caret.position.col
        {
            if let Some(preedit) = preedit_opt {
                self.insert_preedit_chars(
                    preedit,
                    line_row_num,
                    self.main_caret.position.col,
                    state,
                    indent,
                );
                state.main_caret_pos.row = state.phisical_row;
                state.main_caret_pos.col = state.phisical_col;
            } else {
                state.main_caret_pos.row = state.phisical_row;
                state.main_caret_pos.col = state.phisical_col;
            }
            state.preedit_injected = true;
        }
    }

    fn push_buffer_char(
        &self,
        state: &mut LayoutState,
        buffer_char: &BufferChar,
        indent: usize,
        can_break_before: bool,
    ) {
        let char_width = self.width_resolver.resolve_width(buffer_char.c);
        let is_line_head = buffer_char.position.col == 0;
        let can_break_before_current = can_break_before
            && !self
                .line_boundary_prohibited_chars
                .start
                .contains(&buffer_char.c);

        if can_break_before_current {
            state.last_break_candidate = Some(RowBreakCandidate {
                row: state.phisical_row,
                col: state.phisical_col,
                char_index: state.chars.len(),
            });
        }

        self.try_backtrack_wrap(state, char_width, indent, buffer_char.c);

        let before_apply_row = state.phisical_row;
        self.apply_line_break_rules(
            buffer_char.c,
            char_width,
            is_line_head,
            can_break_before,
            &mut state.phisical_row,
            &mut state.phisical_col,
            indent,
        );
        if state.phisical_row != before_apply_row {
            state.current_row_char_start_index = state.chars.len();
            state.last_break_candidate = None;
            state.is_soft_wrapped_row = true;
        }

        let should_trim_soft_wrapped_line_head_whitespace = state.is_soft_wrapped_row
            && state.phisical_col == indent
            && buffer_char.c.is_whitespace();
        let drawn_char_width = if should_trim_soft_wrapped_line_head_whitespace {
            0
        } else {
            char_width
        };

        let phisical_position = PhysicalPosition {
            row: state.phisical_row,
            col: state.phisical_col,
        };

        self.update_caret_position(
            &mut state.main_caret_pos,
            &self.main_caret,
            buffer_char,
            state.phisical_row,
            state.phisical_col,
            drawn_char_width,
        );
        if let Some(mark) = state.mark_pos.as_mut()
            && let Some(mark_caret) = self.mark
        {
            self.update_caret_position(
                mark,
                &mark_caret,
                buffer_char,
                state.phisical_row,
                state.phisical_col,
                drawn_char_width,
            );
        }

        if should_trim_soft_wrapped_line_head_whitespace {
            return;
        }

        state.chars.push((*buffer_char, phisical_position));

        state.phisical_col += drawn_char_width;
    }

    fn try_backtrack_wrap(
        &self,
        state: &mut LayoutState,
        char_width: usize,
        indent: usize,
        c: char,
    ) {
        if state.phisical_col + char_width <= self.max_line_width {
            return;
        }
        if c.is_whitespace() {
            return;
        }
        if self.line_boundary_prohibited_chars.start.contains(&c) {
            return;
        }

        let Some(candidate) = state.last_break_candidate else {
            return;
        };
        if candidate.row != state.phisical_row {
            return;
        }
        if candidate.char_index <= state.current_row_char_start_index {
            return;
        }
        if candidate.col <= indent {
            return;
        }

        let old_row = state.phisical_row;
        let new_row = old_row + 1;
        let split_col = candidate.col;

        for (_, pos) in state.chars.iter_mut().skip(candidate.char_index) {
            let rel_col = pos.col.saturating_sub(split_col);
            pos.row = new_row;
            pos.col = indent + rel_col;
        }
        for (_, pos) in state.preedit_chars.iter_mut() {
            if pos.row == old_row && pos.col >= split_col {
                let rel_col = pos.col.saturating_sub(split_col);
                pos.row = new_row;
                pos.col = indent + rel_col;
            }
        }

        Self::shift_position_after_backtrack(
            &mut state.main_caret_pos,
            old_row,
            split_col,
            new_row,
            indent,
        );
        if let Some(mark_pos) = state.mark_pos.as_mut() {
            Self::shift_position_after_backtrack(mark_pos, old_row, split_col, new_row, indent);
        }

        state.phisical_row = new_row;
        state.phisical_col = indent + state.phisical_col.saturating_sub(split_col);
        state.current_row_char_start_index = candidate.char_index;
        state.last_break_candidate = None;
        state.is_soft_wrapped_row = true;
    }

    fn shift_position_after_backtrack(
        pos: &mut PhysicalPosition,
        old_row: usize,
        split_col: usize,
        new_row: usize,
        indent: usize,
    ) {
        if pos.row == old_row && pos.col >= split_col {
            pos.row = new_row;
            pos.col = indent + pos.col.saturating_sub(split_col);
        }
    }

    fn try_insert_preedit_at_line_end(
        &self,
        state: &mut LayoutState,
        line_row_num: usize,
        line_char_len: usize,
        indent: usize,
        is_caret_row: bool,
        preedit_opt: Option<&str>,
    ) {
        if is_caret_row && !state.preedit_injected && self.main_caret.position.col >= line_char_len
        {
            if let Some(preedit) = preedit_opt {
                self.insert_preedit_chars(
                    preedit,
                    line_row_num,
                    self.main_caret.position.col,
                    state,
                    indent,
                );
                state.main_caret_pos.row = state.phisical_row;
                state.main_caret_pos.col = state.phisical_col;
            } else {
                state.main_caret_pos.row = state.phisical_row;
                state.main_caret_pos.col = state.phisical_col;
            }
            state.preedit_injected = true;
        }
    }

    fn calc_indent(&self, line_string: &str) -> usize {
        let mut list_indent_pattern = DEFAULT_LIST_INDENT_PATTERN.to_vec();
        list_indent_pattern.sort_by(|l, r| l.len().cmp(&r.len()).reverse());
        for pattern in list_indent_pattern {
            if line_string.trim_start().starts_with(pattern) {
                let space_num = line_string.find(pattern).unwrap();
                let space_size = line_string[0..space_num]
                    .chars()
                    .map(|c| self.width_resolver.resolve_width(c))
                    .sum::<usize>();
                let pattern_size = pattern
                    .chars()
                    .map(|c| self.width_resolver.resolve_width(c))
                    .sum::<usize>();
                return space_size + pattern_size;
            }
        }
        if let Some(indent) = self.calc_speaker_indent(line_string) {
            return indent;
        }
        0
    }

    fn calc_speaker_indent(&self, line_string: &str) -> Option<usize> {
        let trimmed = line_string.trim_start();

        let (name, separator) = SPEAKER_SEPARATORS
            .iter()
            .filter_map(|sep| trimmed.split_once(sep).map(|(name, _)| (name, *sep)))
            .min_by_key(|(name, _)| name.len())?;

        if name.is_empty()
            || name.chars().count() > MAX_SPEAKER_NAME_CHARS
            || name
                .chars()
                .any(|c| SPEAKER_NAME_FORBIDDEN_CHARS.contains(&c))
        {
            return None;
        }

        let leading = line_string.len() - trimmed.len();
        let prefix_end = leading + name.len() + separator.len();
        let indent = line_string[0..prefix_end]
            .chars()
            .map(|c| self.width_resolver.resolve_width(c))
            .sum();
        Some(indent)
    }

    fn update_caret_position(
        &self,
        caret_pos: &mut PhysicalPosition,
        caret: &Caret,
        buffer_char: &BufferChar,
        phisical_row: usize,
        phisical_col: usize,
        char_width: usize,
    ) {
        let caret = caret.position;
        let buffer_char = buffer_char.position;
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

    #[allow(clippy::too_many_arguments)]
    fn apply_line_break_rules(
        &self,
        c: char,
        char_width: usize,
        is_line_head: bool,
        can_break_before: bool,
        phisical_row: &mut usize,
        phisical_col: &mut usize,
        indent: usize,
    ) {
        if is_line_head {
            return;
        }

        if can_break_before
            && !self.line_boundary_prohibited_chars.start.contains(&c)
            && *phisical_col >= self.max_line_width
        {
            *phisical_row += 1;
            *phisical_col = indent;
            return;
        }

        let is_line_end_prohibited = self.line_boundary_prohibited_chars.end.contains(&c);
        let new_col = *phisical_col + char_width;

        if new_col < self.max_line_width
            || (new_col <= self.max_line_width && !is_line_end_prohibited)
        {
            return;
        }

        if is_line_end_prohibited && new_col == self.max_line_width {
            *phisical_row += 1;
            *phisical_col = indent;
            return;
        }

        let should_break = (!self.line_boundary_prohibited_chars.start.contains(&c)
            && (can_break_before || self.max_line_width <= *phisical_col))
            || self.max_line_width < *phisical_col;

        if should_break {
            *phisical_row += 1;
            *phisical_col = indent;
        }
    }

    fn insert_preedit_chars(
        &self,
        preedit: &str,
        line_row_num: usize,
        caret_col: usize,
        state: &mut LayoutState,
        indent: usize,
    ) {
        let mut prev_row = state.phisical_row;
        let mut logical_line = line_row_num;
        let mut logical_col = caret_col;
        let preedit_break_before_chars = self.collect_break_before_chars(preedit);

        for (i, c) in preedit.chars().enumerate() {
            let char_width = self.width_resolver.resolve_width(c);
            let is_line_head = (caret_col == 0 && i == 0) || (state.phisical_row > prev_row);
            let can_break_before = preedit_break_before_chars.get(i).copied().unwrap_or(false);

            self.apply_line_break_rules(
                c,
                char_width,
                is_line_head,
                can_break_before,
                &mut state.phisical_row,
                &mut state.phisical_col,
                indent,
            );

            let phisical_position = PhysicalPosition {
                row: state.phisical_row,
                col: state.phisical_col,
            };

            if state.phisical_row > prev_row {
                logical_line += 1;
                logical_col = indent;
            }

            let logical_pos = CellPosition::new(logical_line, logical_col);
            state.preedit_chars.push((
                BufferChar {
                    position: logical_pos,
                    c,
                },
                phisical_position,
            ));

            state.phisical_col += char_width;
            logical_col += 1;
            prev_row = state.phisical_row;
        }
    }

    fn collect_break_before_chars(&self, text: &str) -> Vec<bool> {
        let char_len = text.chars().count();
        let mut break_before_chars = vec![false; char_len + 1];
        let segmenter = LineSegmenter::new_auto(LineBreakOptions::default());
        for byte_idx in segmenter.segment_str(text) {
            let char_idx = text[..byte_idx].chars().count();
            if char_idx <= char_len {
                break_before_chars[char_idx] = true;
            }
        }
        break_before_chars
    }
}

struct LayoutState {
    chars: Vec<(BufferChar, PhysicalPosition)>,
    preedit_chars: Vec<(BufferChar, PhysicalPosition)>,
    phisical_row: usize,
    phisical_col: usize,
    main_caret_pos: PhysicalPosition,
    mark_pos: Option<PhysicalPosition>,
    preedit_injected: bool,
    current_row_char_start_index: usize,
    last_break_candidate: Option<RowBreakCandidate>,
    is_soft_wrapped_row: bool,
}

impl LayoutState {
    fn new(mark: Option<Caret>) -> Self {
        Self {
            chars: Vec::new(),
            preedit_chars: Vec::new(),
            phisical_row: 0,
            phisical_col: 0,
            main_caret_pos: PhysicalPosition { row: 0, col: 0 },
            mark_pos: mark.map(|_| PhysicalPosition { row: 0, col: 0 }),
            preedit_injected: false,
            current_row_char_start_index: 0,
            last_break_candidate: None,
            is_soft_wrapped_row: false,
        }
    }

    fn into_layout(self) -> PhysicalLayout {
        PhysicalLayout {
            chars: self.chars,
            preedit_chars: self.preedit_chars,
            main_caret_pos: self.main_caret_pos,
            mark_pos: self.mark_pos,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct RowBreakCandidate {
    row: usize,
    col: usize,
    char_index: usize,
}

const DEFAULT_LIST_INDENT_PATTERN: [&str; 17] = [
    "* ", "* [ ] ", "* [x] ", "- ", "- [ ] ", "- [x] ", "> ", "・", "1. ", "2. ", "3. ", "4. ",
    "5. ", "6. ", "7. ", "8. ", "9. ",
];

const SPEAKER_SEPARATORS: [&str; 4] = [": ", "： ", "：　", ":　"];

const MAX_SPEAKER_NAME_CHARS: usize = 20;

const SPEAKER_NAME_FORBIDDEN_CHARS: [char; 12] = [
    '。', '、', '．', '，', '！', '？', '!', '?', '「', '」', '『', '』',
];

#[cfg(test)]
mod tests {
    use super::*;

    use text_buffer::action::EditorOperation;
    use text_buffer::editor::Editor;

    struct TestWidthResolver;

    impl CharWidthResolver for TestWidthResolver {
        fn resolve_width(&self, c: char) -> usize {
            if c.is_ascii() { 1 } else { 2 }
        }
    }

    struct AsciiWideXResolver;

    impl CharWidthResolver for AsciiWideXResolver {
        fn resolve_width(&self, c: char) -> usize {
            if c == 'X' { 2 } else { 1 }
        }
    }

    fn run_ops(ops: &[EditorOperation]) -> Editor {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut editor = Editor::new(sender.clone());
        ops.iter().for_each(|op| editor.operation(op));
        let _ = receiver.try_iter().collect::<Vec<_>>();
        editor
    }

    #[test]
    fn test_calc_phisical_layout() {
        struct TestCase {
            input: Vec<EditorOperation>,
            output: String,
            max_width: usize,
            main_caret_pos: PhysicalPosition,
            mark_pos: Option<PhysicalPosition>,
        }
        let cases = [
            TestCase {
                input: vec![EditorOperation::InsertString(
                    "ABCDE\nFGHIJ\nKLMNO".to_string(),
                )],
                output: "ABCD\nE\nFGHI\nJ\nKLMN\nO".to_string(),
                max_width: 4,
                main_caret_pos: PhysicalPosition { row: 5, col: 1 },
                mark_pos: None,
            },
            TestCase {
                input: vec![EditorOperation::InsertString(
                    "ABCDE\nFGHIJ\nKLMNO".to_string(),
                )],
                output: "ABCDE\nFGHIJ\nKLMNO".to_string(),
                max_width: 10,
                main_caret_pos: PhysicalPosition { row: 2, col: 5 },
                mark_pos: None,
            },
            TestCase {
                input: vec![EditorOperation::InsertString("日本の四季折々".to_string())],
                output: "日本の四季\n折々".to_string(),
                max_width: 10,
                main_caret_pos: PhysicalPosition { row: 1, col: 4 },
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
                main_caret_pos: PhysicalPosition { row: 1, col: 0 },
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
                main_caret_pos: PhysicalPosition { row: 1, col: 0 },
                mark_pos: Some(PhysicalPosition { row: 0, col: 1 }),
            },
        ];

        for (idx, case) in cases.iter().enumerate() {
            let editor = run_ops(&case.input);
            let layout = calc_phisical_layout(
                &editor,
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
                output: "Hello, World!\nAnd you.".to_string(),
                prohibited_chars: LineBoundaryProhibitedChars::default(),
                max_width: 12,
            },
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
            let editor = run_ops(&case.input);
            let layout = calc_phisical_layout(
                &editor,
                case.max_width,
                &case.prohibited_chars,
                Arc::new(TestWidthResolver),
                None,
            );
            assert_eq!(layout.to_string(), case.output);
        }
    }

    #[test]
    fn test_wrap_defers_inside_unbreakable_word_when_wide_char_overflows() {
        let editor = run_ops(&[EditorOperation::InsertString("abcXdef".to_string())]);

        let layout = calc_phisical_layout(
            &editor,
            4,
            &LineBoundaryProhibitedChars::new(vec![], vec![]),
            Arc::new(AsciiWideXResolver),
            None,
        );

        assert_eq!(layout.to_string(), "abcX\ndef");
    }

    #[test]
    fn test_wrap_english_sentence_on_word_boundaries() {
        let editor = run_ops(&[EditorOperation::InsertString(
            "tiny elephant runs over a bridge slowly".to_string(),
        )]);

        let layout = calc_phisical_layout(
            &editor,
            10,
            &LineBoundaryProhibitedChars::new(vec![], vec![]),
            Arc::new(TestWidthResolver),
            None,
        );

        assert_eq!(
            layout.to_string(),
            "tiny \nelephant \nruns over \na bridge \nslowly"
        );
    }

    #[test]
    fn test_soft_wrap_trims_wrapped_line_head_space() {
        let editor = run_ops(&[EditorOperation::InsertString(
            "Hello, World! And you.".to_string(),
        )]);

        let layout = calc_phisical_layout(
            &editor,
            12,
            &LineBoundaryProhibitedChars::default(),
            Arc::new(TestWidthResolver),
            None,
        );
        assert_eq!(layout.to_string(), "Hello, World!\nAnd you.");
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
            TestCase {
                input: vec![EditorOperation::InsertString(
                    "三苫: 私はこう思います。".to_string(),
                )],
                output: "三苫: 私は\n      こう\n      思い\n      ます。".to_string(),
                prohibited_chars: LineBoundaryProhibitedChars::default(),
                max_width: 10,
            },
        ];

        for case in cases.iter() {
            let editor = run_ops(&case.input);
            let layout = calc_phisical_layout(
                &editor,
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
            TestCase {
                input: vec![
                    EditorOperation::InsertString("テストテストテストテスト".to_string()),
                    EditorOperation::BufferHead,
                    EditorOperation::Forward,
                    EditorOperation::Forward,
                    EditorOperation::Forward,
                    EditorOperation::Forward,
                ],
                preedit_string: "かな".to_string(),
                output: "テストテストテストテスト".to_string(),
                prohibited_chars: LineBoundaryProhibitedChars::default(),
                max_width: 100,
            },
            TestCase {
                input: vec![EditorOperation::InsertString("inline".to_string())],
                preedit_string: "ダダダダダダ".to_string(),
                output: "inline".to_string(),
                prohibited_chars: LineBoundaryProhibitedChars::default(),
                max_width: 10,
            },
            TestCase {
                input: vec![EditorOperation::InsertString("text".to_string())],
                preedit_string: "ダダ。ダ".to_string(),
                output: "text".to_string(),
                prohibited_chars: LineBoundaryProhibitedChars::default(),
                max_width: 8,
            },
            TestCase {
                input: vec![EditorOperation::InsertString("x".to_string())],
                preedit_string: "ダダ。。".to_string(),
                output: "x".to_string(),
                prohibited_chars: LineBoundaryProhibitedChars::default(),
                max_width: 6,
            },
        ];

        for case in cases.iter() {
            let editor = run_ops(&case.input);
            let layout = calc_phisical_layout(
                &editor,
                case.max_width,
                &case.prohibited_chars,
                Arc::new(TestWidthResolver),
                Some(case.preedit_string.clone()),
            );
            assert_eq!(layout.to_string(), case.output);

            if let Some((last_preedit_char, last_preedit_pos)) = layout.preedit_chars.last() {
                let expected_main_caret_pos = PhysicalPosition {
                    row: last_preedit_pos.row,
                    col: last_preedit_pos.col
                        + TestWidthResolver.resolve_width(last_preedit_char.c),
                };
                assert_eq!(
                    layout.main_caret_pos, expected_main_caret_pos,
                    "main caret should be placed right after preedit"
                );
            }
        }
    }
}
