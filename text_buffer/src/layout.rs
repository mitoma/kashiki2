use std::fmt::Display;
use std::sync::Arc;

use crate::buffer::{Buffer, BufferChar};
use crate::caret::Caret;

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

pub(super) struct PhysicalLayoutCalculator<'a> {
    buffer: &'a Buffer,
    main_caret: &'a Caret,
    mark: Option<Caret>,
    max_line_width: usize,
    line_boundary_prohibited_chars: &'a LineBoundaryProhibitedChars,
    width_resolver: Arc<dyn CharWidthResolver>,
    preedit_string: Option<String>,
}

impl<'a> PhysicalLayoutCalculator<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        buffer: &'a Buffer,
        main_caret: &'a Caret,
        mark: Option<Caret>,
        max_line_width: usize,
        line_boundary_prohibited_chars: &'a LineBoundaryProhibitedChars,
        width_resolver: Arc<dyn CharWidthResolver>,
        preedit_string: Option<String>,
    ) -> Self {
        Self {
            buffer,
            main_caret,
            mark,
            max_line_width,
            line_boundary_prohibited_chars,
            width_resolver,
            preedit_string,
        }
    }

    pub(super) fn calc(&self) -> PhysicalLayout {
        let mut state = LayoutState::new(self.mark);
        let preedit_opt = self.preedit_string.as_deref();

        for line in &self.buffer.lines {
            state.phisical_col = 0;
            let is_caret_row = self.main_caret.position.row == line.row_num;

            if line.chars.is_empty() {
                self.handle_empty_line(&mut state, line.row_num, is_caret_row, preedit_opt);
            }

            let indent = self.calc_indent(&line.to_line_string());

            for buffer_char in &line.chars {
                self.try_insert_preedit_before_char(
                    &mut state,
                    buffer_char,
                    line.row_num,
                    indent,
                    is_caret_row,
                    preedit_opt,
                );

                self.push_buffer_char(&mut state, buffer_char, indent);
            }

            self.try_insert_preedit_at_line_end(
                &mut state,
                line.row_num,
                line.chars.len(),
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
            state.main_caret_pos.row = state.phisical_row;
            state.main_caret_pos.col = state.phisical_col;

            if let Some(preedit) = preedit_opt {
                self.insert_preedit_chars(
                    preedit,
                    line_row_num,
                    self.main_caret.position.col,
                    state,
                    indent,
                );
            }
            state.preedit_injected = true;
        }
    }

    fn push_buffer_char(&self, state: &mut LayoutState, buffer_char: &BufferChar, indent: usize) {
        let char_width = self.width_resolver.resolve_width(buffer_char.c);
        let is_line_head = buffer_char.position.col == 0;
        self.apply_line_break_rules(
            buffer_char.c,
            char_width,
            is_line_head,
            &mut state.phisical_row,
            &mut state.phisical_col,
            indent,
        );

        let phisical_position = PhysicalPosition {
            row: state.phisical_row,
            col: state.phisical_col,
        };
        state.chars.push((*buffer_char, phisical_position));

        self.update_caret_position(
            &mut state.main_caret_pos,
            self.main_caret,
            buffer_char,
            state.phisical_row,
            state.phisical_col,
            char_width,
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
                char_width,
            );
        }

        state.phisical_col += char_width;
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
            state.main_caret_pos.row = state.phisical_row;
            state.main_caret_pos.col = state.phisical_col;
            if let Some(preedit) = preedit_opt {
                self.insert_preedit_chars(
                    preedit,
                    line_row_num,
                    self.main_caret.position.col,
                    state,
                    indent,
                );
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
        0
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

    fn apply_line_break_rules(
        &self,
        c: char,
        char_width: usize,
        is_line_head: bool,
        phisical_row: &mut usize,
        phisical_col: &mut usize,
        indent: usize,
    ) {
        if is_line_head {
            return;
        }

        if *phisical_col + char_width >= self.max_line_width
            && self.line_boundary_prohibited_chars.end.contains(&c)
        {
            *phisical_row += 1;
            *phisical_col = indent;
        } else if *phisical_col + char_width > self.max_line_width {
            if self.line_boundary_prohibited_chars.start.contains(&c)
                && self.max_line_width >= *phisical_col
            {
            } else {
                *phisical_row += 1;
                *phisical_col = indent;
            }
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

        for (i, c) in preedit.chars().enumerate() {
            let char_width = self.width_resolver.resolve_width(c);
            let is_line_head = (caret_col == 0 && i == 0) || (state.phisical_row > prev_row);

            self.apply_line_break_rules(
                c,
                char_width,
                is_line_head,
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

            let logical_pos = [logical_line, logical_col].into();
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
}

struct LayoutState {
    chars: Vec<(BufferChar, PhysicalPosition)>,
    preedit_chars: Vec<(BufferChar, PhysicalPosition)>,
    phisical_row: usize,
    phisical_col: usize,
    main_caret_pos: PhysicalPosition,
    mark_pos: Option<PhysicalPosition>,
    preedit_injected: bool,
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

const DEFAULT_LIST_INDENT_PATTERN: [&str; 17] = [
    "* ", "* [ ] ", "* [x] ", "- ", "- [ ] ", "- [x] ", "> ", "・", "1. ", "2. ", "3. ", "4. ",
    "5. ", "6. ", "7. ", "8. ", "9. ",
];
