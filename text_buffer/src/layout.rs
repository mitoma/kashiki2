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
    pub fn new(
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

    /// バッファ内のすべての行に対して物理レイアウトを計算する。
    ///
    /// 論理行 (buffer) を物理行 (display) にマッピングし、以下を処理する：
    /// - 折り返しと禁則文字ルール
    /// - caret と mark の物理位置
    /// - preedit 文字列の挿入位置
    pub fn calc(&self) -> PhysicalLayout {
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

    /// 空行 (文字なし) でのレイアウト処理。
    ///
    /// isEmpty な行でも caret/mark 位置を記録し、必要に応じて preedit を挿入する。
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

    /// caret が buffer_char の位置にある場合、その直前に preedit を挿入する。
    ///
    /// 入力中テキスト (preedit) は caret 位置から始まり、main caret は preedit の末尾直後に置く。
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

    /// buffer_char を物理位置に配置し、caret/mark 位置と折り返し規則を適用する。
    ///
    /// 禁則文字を考慮した改行判定、主caret と mark の位置追跡を行った上で、
    /// 文字を物理レイアウト出力に追加する。
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

    /// caret が論理行末 (行の文字数以上の位置) にある場合、行末に preedit を挿入する。
    ///
    /// 行の全文字走査後、caret が行末にある場合のみこの処理が実行される。
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

    /// 行の先頭パターン ("- ", "* [ ] " など) に基づいて、折り返し時のインデント幅を計算する。
    ///
    /// 箇条書き行では折り返し後に同じインデントレベルの位置から再開する。
    /// パターンは長い順で評価し、より具体的なマッチを優先する。
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

    /// 論理位置の caret が buffer_char と一致または直後の場合、その物理位置を記録する。
    ///
    /// caret.col == buffer_char.col: caret は文字の直前
    /// caret.col == buffer_char.col + 1: caret は文字の直後
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

    /// 禁則文字ルール (行頭禁則、行末禁則) を考慮して改行判定を行う。
    ///
    /// max_line_width を超える場合、以下の条件で改行を決定する：
    /// - 行末禁則文字が max_line_width 直前なら、その前で改行
    /// - 行頭禁則文字なら超過を許容
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

        let is_line_end_prohibited = self.line_boundary_prohibited_chars.end.contains(&c);
        let new_col = *phisical_col + char_width;

        // 幅内に完全に収まり、かつ行末禁則でない場合はスキップ
        if new_col < self.max_line_width
            || (new_col <= self.max_line_width && !is_line_end_prohibited)
        {
            return;
        }

        // 行末禁則文字が max_line_width ちょうどに来る場合は改行
        if is_line_end_prohibited && new_col == self.max_line_width {
            *phisical_row += 1;
            *phisical_col = indent;
            return;
        }

        // 幅を超える場合の判定
        let should_break = !self.line_boundary_prohibited_chars.start.contains(&c)
            || self.max_line_width < *phisical_col;

        if should_break {
            *phisical_row += 1;
            *phisical_col = indent;
        }
    }

    /// preedit 文字列を物理レイアウトに挿入し、折り返しと論理位置を管理する。
    ///
    /// preedit は複数行にまたがることがあり、その場合も論理行と物理行を正しく対応付ける。
    /// インデントも折り返し後に継承される。
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
