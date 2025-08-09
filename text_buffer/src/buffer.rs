use std::{ops::RangeBounds, sync::mpsc::Sender};

use crate::{caret::Caret, char_type::CharType, editor::ChangeEvent};

pub struct Buffer {
    pub lines: Vec<BufferLine>,
    sender: Sender<ChangeEvent>,
}

impl Buffer {
    pub(crate) fn new(sender: Sender<ChangeEvent>) -> Self {
        Self {
            lines: vec![BufferLine::default()],
            sender,
        }
    }

    pub(crate) fn to_buffer_string(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.to_line_string())
            .collect::<Vec<String>>()
            .join("\n")
    }

    pub(crate) fn insert_string(&mut self, caret: &mut Caret, string: String) {
        let mut iter = string.split("\r\n").flat_map(|line| line.split('\n'));
        let first_line = match iter.next() {
            Some(line) => line,
            None => return,
        };
        first_line.chars().for_each(|c| self.insert_char(caret, c));
        iter.for_each(|line| {
            self.insert_enter(caret);
            line.chars().for_each(|c| self.insert_char(caret, c))
        })
    }

    pub(crate) fn insert_char(&mut self, caret: &mut Caret, c: char) {
        if let Some(line) = self.lines.get_mut(caret.position.row) {
            line.insert_char(caret.position.col, c, &self.sender);
            caret.move_to(caret.position.next_col(), &self.sender);
        }
    }

    pub(crate) fn insert_enter(&mut self, caret: &mut Caret) {
        if let Some(line) = self.lines.get_mut(caret.position.row)
            && let Some(mut next_line) = line.insert_enter(caret.position.col)
        {
            self.lines
                .iter_mut()
                .skip(caret.position.row + 1)
                .rev()
                .for_each(|line| line.update_position(line.row_num + 1, &self.sender));
            next_line.update_position(caret.position.row + 1, &self.sender);
            self.lines.insert(caret.position.row + 1, next_line);
            caret.move_to(caret.position.next_row_first(), &self.sender);
        }
    }

    fn update_position(&mut self) {
        (0..).zip(self.lines.iter_mut()).for_each(|(i, l)| {
            l.update_position(i, &self.sender);
        })
    }

    pub(crate) fn head(&self, caret: &mut Caret) {
        caret.move_to(caret.position.with_col(0), &self.sender);
    }

    pub(crate) fn last(&self, caret: &mut Caret) {
        if let Some(line) = self.lines.get(caret.position.row) {
            caret.move_to(caret.position.with_col(line.chars.len()), &self.sender);
        }
    }

    pub(crate) fn back(&mut self, caret: &mut Caret) {
        match (self.is_line_head(caret), self.is_buffer_head(caret)) {
            // 行頭かつバッファの先頭であればなにもしない
            (true, true) => {}
            // 行頭であれば前の行の末尾に移動
            (true, false) => {
                self.previous(caret);
                self.last(caret);
            }
            // 行頭でなければ前の文字に移動
            (false, true) | (false, false) => {
                caret.move_to(caret.position.prev_col(), &self.sender)
            }
        }
    }

    pub(crate) fn back_word(&mut self, caret: &mut Caret) {
        match (self.is_line_head(caret), self.is_buffer_head(caret)) {
            // 行頭かつバッファの先頭であればなにもしない
            (true, true) => {}
            // 行頭であれば前の行の末尾に移動
            (true, false) => {
                self.previous(caret);
                self.last(caret);
            }
            // 行頭でなければ前のワードに移動
            (false, true) | (false, false) => {
                // 前の word の先頭に移動する
                if let Some(line) = self.lines.get(caret.position.row) {
                    let mut chars = line
                        .chars
                        .iter()
                        .rev()
                        .skip(line.chars.len() - caret.position.col);
                    let mut next_col = caret.position.col;
                    let mut current_char_type = CharType::from_char(chars.next().unwrap().c);
                    for c in chars {
                        next_col -= 1;
                        let next_char_type = CharType::from_char(c.c);
                        if current_char_type.skip_word(&next_char_type) {
                            current_char_type = next_char_type;
                            continue;
                        }
                        caret.move_to(caret.position.with_col(next_col), &self.sender);
                        return;
                    }
                    // ループを抜けた場合は行頭にいく
                    self.head(caret);
                }
            }
        }
    }

    pub(crate) fn forward(&mut self, caret: &mut Caret) {
        match (self.is_line_last(caret), self.is_buffer_last(caret)) {
            // 行末かつバッファの最後であればなにもしない
            (true, true) => {}
            // 行末であれば次の行の先頭に移動
            (true, false) => {
                self.next(caret);
                self.head(caret);
            }
            // 行末でなければ次の文字に移動
            (false, true) | (false, false) => {
                caret.move_to(caret.position.next_col(), &self.sender)
            }
        }
    }

    pub(crate) fn forward_word(&mut self, caret: &mut Caret) {
        match (self.is_line_last(caret), self.is_buffer_last(caret)) {
            // 行末かつバッファの最後であればなにもしない
            (true, true) => {}
            // 行末であれば次の行の先頭に移動
            (true, false) => {
                self.next(caret);
                self.head(caret);
            }
            // 行末でなければ次のワードに移動
            (false, true) | (false, false) => {
                // 次の word の先頭に移動する
                if let Some(line) = self.lines.get(caret.position.row) {
                    let mut chars = line.chars.iter().skip(caret.position.col);
                    let mut next_col = caret.position.col;
                    let mut current_char_type = CharType::from_char(chars.next().unwrap().c);
                    for c in chars {
                        next_col += 1;
                        let next_char_type = CharType::from_char(c.c);
                        if current_char_type.skip_word(&next_char_type) {
                            current_char_type = next_char_type;
                            continue;
                        }
                        caret.move_to(caret.position.with_col(next_col), &self.sender);
                        return;
                    }
                    // ループを抜けた場合は行末にいく
                    self.last(caret);
                }
            }
        }
    }

    pub(crate) fn previous(&mut self, caret: &mut Caret) {
        if !self.is_buffer_head(caret) {
            caret.move_to(caret.position.prev_row(), &self.sender);
            if self.is_line_last(caret) {
                // 前行が短い場合に Caret 位置を調整
                self.last(caret)
            }
        }
    }

    pub(crate) fn next(&self, caret: &mut Caret) {
        if !self.is_buffer_last(caret) {
            caret.move_to(caret.position.next_row(), &self.sender);
            if self.is_line_last(caret) {
                // 次行が短い場合に Caret 位置を調整
                self.last(caret)
            }
        }
    }

    pub(crate) fn buffer_head(&self, caret: &mut Caret) {
        caret.move_to([0, 0].into(), &self.sender);
    }

    pub(crate) fn buffer_last(&self, caret: &mut Caret) {
        if let Some(last_line) = self.lines.last() {
            caret.move_to(
                [last_line.row_num, last_line.chars.len()].into(),
                &self.sender,
            );
        }
    }

    fn is_buffer_head(&self, caret: &Caret) -> bool {
        caret.position.row == 0
    }

    fn is_buffer_last(&self, caret: &Caret) -> bool {
        caret.position.row == self.lines.len() - 1
    }

    fn is_line_head(&self, caret: &Caret) -> bool {
        caret.position.col == 0
    }

    fn is_line_last(&self, caret: &Caret) -> bool {
        if let Some(line_length) = self
            .lines
            .get(caret.position.row)
            .map(|line| line.chars.len())
        {
            caret.position.col >= line_length
        } else {
            false
        }
    }

    pub(crate) fn backspace(&mut self, caret: &mut Caret) -> RemovedChar {
        if self.is_buffer_head(caret) && self.is_line_head(caret) {
            RemovedChar::None
        } else {
            self.back(caret);
            self.delete(caret)
        }
    }

    pub(crate) fn delete(&mut self, caret: &Caret) -> RemovedChar {
        if self.is_line_last(caret) {
            if !self.is_buffer_last(caret) {
                let next_line = self.lines.remove(caret.position.row + 1);
                let current_line = self.lines.get_mut(caret.position.row).unwrap();
                current_line.join(next_line, &self.sender);
                self.update_position();
                RemovedChar::Enter
            } else {
                RemovedChar::None
            }
        } else if let Some(line) = self.lines.get_mut(caret.position.row) {
            line.remove_char(caret.position.col, &self.sender)
        } else {
            RemovedChar::None
        }
    }

    pub(crate) fn copy_string(&self, mark_caret: &Caret, current_caret: &Caret) -> String {
        if mark_caret.position == current_caret.position {
            return String::new();
        }
        let (start, end) = if mark_caret < current_caret {
            (mark_caret, current_caret)
        } else {
            (current_caret, mark_caret)
        };
        let mut result = String::new();
        if start.position.row == end.position.row {
            if let Some(line) = self.lines.get(start.position.row) {
                result.push_str(&line.substring(start.position.col..end.position.col));
            }
        } else {
            if let Some(start_line) = self.lines.get(start.position.row) {
                result.push_str(&start_line.substring(start.position.col..));
                result.push('\n');
            }
            for line in self
                .lines
                .iter()
                .skip(start.position.row + 1)
                .take(end.position.row - start.position.row - 1)
            {
                result.push_str(&line.to_line_string());
                result.push('\n');
            }
            if let Some(end_line) = self.lines.get(end.position.row) {
                result.push_str(&end_line.substring(..end.position.col));
            }
        }
        result
    }

    #[inline]
    fn byte_to_char_index(s: &str, byte_index: usize) -> Option<usize> {
        s.char_indices()
            .enumerate()
            .find_map(|(char_index, (byte_pos, _))| {
                if byte_pos == byte_index {
                    Some(char_index)
                } else {
                    None
                }
            })
    }

    #[inline]
    fn highlight_positions(&self, highlight_string: &str) -> Vec<CellPosition> {
        let mut result = Vec::new();
        for line in &self.lines {
            let line_string = line.to_line_string();
            let mut slice_start = 0;
            let mut search_target = &line_string[slice_start..];

            while let Some(idx) = search_target.find(highlight_string) {
                let start = slice_start + idx;
                let end = start + highlight_string.len();
                result.push(CellPosition {
                    row: line.row_num,
                    col: Self::byte_to_char_index(&line_string, start).unwrap(),
                });
                slice_start = end;
                search_target = &line_string[slice_start..];
            }
        }
        result
    }

    pub fn highlight(&self, highlight_string: &str) {
        self.highlight_positions(highlight_string)
            .iter()
            .flat_map(|pos| {
                self.lines[pos.row].chars[pos.col..(pos.col + highlight_string.chars().count())]
                    .to_vec()
            })
            .for_each(|buffer_char| {
                self.sender
                    .send(ChangeEvent::SelectChar(buffer_char))
                    .unwrap();
            });
    }

    pub fn unhighlight(&self, highlight_string: &str) {
        self.highlight_positions(highlight_string)
            .iter()
            .flat_map(|pos| {
                self.lines[pos.row].chars[pos.col..(pos.col + highlight_string.chars().count())]
                    .to_vec()
            })
            .for_each(|buffer_char| {
                self.sender
                    .send(ChangeEvent::UnSelectChar(buffer_char))
                    .unwrap();
            });
    }

    pub fn move_to_next(&mut self, caret: &mut Caret, keyword: &str) {
        let positions = self.highlight_positions(keyword);
        let _ = positions
            .iter()
            .find(|pos| pos > &&caret.position)
            .or(positions.first())
            .map(|pos| {
                caret.move_to(*pos, &self.sender);
            });
    }

    pub fn move_to_previous(&mut self, caret: &mut Caret, keyword: &str) {
        let positions = self.highlight_positions(keyword);
        let _ = positions
            .iter()
            .rev()
            .find(|pos| pos < &&caret.position)
            .or(positions.last())
            .map(|pos| {
                caret.move_to(*pos, &self.sender);
            });
    }
}

#[derive(Default)]
pub struct BufferLine {
    // 0 origin
    pub(crate) row_num: usize,
    pub(crate) chars: Vec<BufferChar>,
}

impl BufferLine {
    fn from_chars(chars: Vec<BufferChar>) -> BufferLine {
        BufferLine { row_num: 0, chars }
    }

    fn update_position(&mut self, row_num: usize, sender: &Sender<ChangeEvent>) {
        self.row_num = row_num;
        (0..).zip(self.chars.iter_mut()).for_each(|(i, c)| {
            c.update_position([row_num, i].into(), sender);
        })
    }

    pub(crate) fn to_line_string(&self) -> String {
        self.chars.iter().map(|c| c.c).collect()
    }

    fn insert_char(&mut self, col: usize, c: char, sender: &Sender<ChangeEvent>) {
        self.chars
            .iter_mut()
            .skip(col)
            .rev()
            .for_each(|c| c.update_position([self.row_num, c.position.col + 1].into(), sender));
        self.chars
            .insert(col, BufferChar::new([self.row_num, col].into(), c, sender))
    }

    fn insert_enter(&mut self, col: usize) -> Option<BufferLine> {
        match self.chars.len() {
            len if len == col => {
                let line = BufferLine {
                    row_num: self.row_num + 1,
                    ..BufferLine::default()
                };
                Some(line)
            }
            len if len > col => {
                let mut line = BufferLine::from_chars(self.chars.split_off(col));
                line.row_num = self.row_num + 1;
                Some(line)
            }
            _ => None,
        }
    }

    fn remove_char(&mut self, col: usize, sender: &Sender<ChangeEvent>) -> RemovedChar {
        let removed = self.chars.remove(col);
        sender.send(ChangeEvent::RemoveChar(removed)).unwrap();
        self.chars
            .iter_mut()
            .skip(col)
            .for_each(|c| c.update_position([self.row_num, c.position.col - 1].into(), sender));
        RemovedChar::Char(removed.c)
    }

    fn join(&mut self, line: BufferLine, sender: &Sender<ChangeEvent>) {
        let current_len = self.chars.len();
        line.chars
            .into_iter()
            .map(|mut c| {
                c.update_position([self.row_num, current_len + c.position.col].into(), sender);
                c
            })
            .for_each(|c| self.chars.push(c))
    }

    fn substring<R>(&self, range: R) -> String
    where
        R: RangeBounds<usize>,
    {
        let start = match range.start_bound() {
            std::ops::Bound::Included(&s) => s,
            std::ops::Bound::Excluded(&s) => s + 1,
            std::ops::Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            std::ops::Bound::Included(&e) => e + 1,
            std::ops::Bound::Excluded(&e) => e,
            std::ops::Bound::Unbounded => self.chars.len(),
        };
        // Caret の位置は Line の長さを超えるケースがあるので、範囲外の場合は Line の最後尾までを返す
        let end = if end > self.chars.len() {
            self.chars.len()
        } else {
            end
        };
        self.chars[start..end].iter().map(|c| c.c).collect()
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct CellPosition {
    // 0 origin
    pub row: usize,
    // 0 origin
    pub col: usize,
}

impl From<[usize; 2]> for CellPosition {
    fn from(value: [usize; 2]) -> Self {
        Self {
            row: value[0],
            col: value[1],
        }
    }
}

impl CellPosition {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }

    pub fn is_same_or_after_on_row(&self, other: &CellPosition) -> bool {
        self.row == other.row && self.col >= other.col
    }

    pub fn in_range(&self, from: CellPosition, to: CellPosition) -> bool {
        let (from, to) = if from < to { (from, to) } else { (to, from) };
        if from.row > self.row || to.row < self.row {
            return false;
        }
        if from.row == self.row && from.col > self.col {
            return false;
        }
        if to.row == self.row && to.col <= self.col {
            return false;
        }
        true
    }

    pub fn next_row(&self) -> Self {
        Self {
            row: self.row + 1,
            col: self.col,
        }
    }

    pub fn prev_row(&self) -> Self {
        Self {
            row: self.row - 1,
            col: self.col,
        }
    }

    // 次の行の先頭
    pub fn next_row_first(&self) -> Self {
        Self {
            row: self.row + 1,
            col: 0,
        }
    }

    pub fn next_col(&self) -> Self {
        Self {
            row: self.row,
            col: self.col + 1,
        }
    }

    pub fn prev_col(&self) -> Self {
        Self {
            row: self.row,
            col: self.col - 1,
        }
    }

    pub fn with_row(&self, row: usize) -> Self {
        Self { row, col: self.col }
    }

    pub fn with_col(&self, col: usize) -> Self {
        Self { row: self.row, col }
    }

    pub fn is_same_row(&self, other: &CellPosition) -> bool {
        self.row == other.row
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct BufferChar {
    pub position: CellPosition,
    pub c: char,
}

impl BufferChar {
    fn new(position: CellPosition, c: char, sender: &Sender<ChangeEvent>) -> Self {
        let instance = Self { position, c };
        sender.send(ChangeEvent::AddChar(instance)).unwrap();
        instance
    }

    fn update_position(&mut self, position: CellPosition, sender: &Sender<ChangeEvent>) {
        if self.position == position {
            return;
        }
        let from = *self;
        self.position = position;
        let event = ChangeEvent::MoveChar { from, to: *self };
        sender.send(event).unwrap();
    }

    // to は含まない
    pub fn in_caret_range(&self, from: Caret, to: Caret) -> bool {
        self.position.in_range(from.position, to.position)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum RemovedChar {
    Char(char),
    Enter,
    None,
}

#[cfg(test)]
mod tests {

    use std::sync::mpsc::channel;

    use crate::{caret::CaretType, editor::ChangeEvent};

    use super::*;

    #[test]
    fn buffer() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let caret = &mut Caret::new([0, 0].into(), &tx);
        let mut sut = Buffer::new(tx.clone());
        assert_eq!(sut.to_buffer_string(), "");
        sut.insert_char(caret, '山');
        assert_eq!(sut.to_buffer_string(), "山");
        assert_eq!(caret.position, [0, 1].into());
        sut.insert_char(caret, '本');
        assert_eq!(sut.to_buffer_string(), "山本");
        assert_eq!(caret.position, [0, 2].into());
        sut.insert_enter(caret);
        assert_eq!(sut.to_buffer_string(), "山本\n");
        assert_eq!(caret.position, [1, 0].into());
        sut.insert_enter(caret);
        assert_eq!(sut.to_buffer_string(), "山本\n\n");
        assert_eq!(caret.position, [2, 0].into());
        sut.insert_enter(&mut Caret::new([100, 100].into(), &tx));
        assert_eq!(sut.to_buffer_string(), "山本\n\n");
    }

    #[test]
    fn buffer_insert_string() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let caret = &mut Caret::new([0, 0].into(), &tx);
        let mut sut = Buffer::new(tx);
        sut.insert_string(caret, "東京は\n今日もいい天気\nだった。".to_string());
        assert_eq!(sut.to_buffer_string(), "東京は\n今日もいい天気\nだった。");
        assert_eq!(caret.position, [2, 4].into());
    }

    #[test]
    fn buffer_position_check() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let mut sut = Buffer::new(tx.clone());
        sut.insert_string(
            &mut Caret::new([0, 0].into(), &tx),
            "あいうえお\nかきくけこ\nさしすせそそ".to_string(),
        );
        // buffer head
        assert!(sut.is_buffer_head(&Caret::new([0, 0].into(), &tx)));
        assert!(sut.is_buffer_head(&Caret::new([0, 4].into(), &tx)));
        assert!(!sut.is_buffer_head(&Caret::new([1, 0].into(), &tx)));

        // buffer last
        assert!(sut.is_buffer_last(&Caret::new([2, 0].into(), &tx)));
        assert!(sut.is_buffer_last(&Caret::new([2, 4].into(), &tx)));
        assert!(!sut.is_buffer_last(&Caret::new([0, 0].into(), &tx)));

        // line head
        assert!(sut.is_line_head(&Caret::new([0, 0].into(), &tx)));
        assert!(sut.is_line_head(&Caret::new([2, 0].into(), &tx)));
        assert!(!sut.is_line_head(&Caret::new([1, 3].into(), &tx)));

        // line last
        assert!(sut.is_line_last(&Caret::new([0, 5].into(), &tx)));
        assert!(sut.is_line_last(&Caret::new([2, 6].into(), &tx)));
        assert!(!sut.is_line_last(&Caret::new([2, 5].into(), &tx)));
    }

    #[test]
    fn buffer_move() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let mut sut = Buffer::new(tx.clone());
        let caret = &mut Caret::new([0, 0].into(), &tx);
        sut.insert_string(caret, "あいうえお\nきかくけここ\nさしすせそ".to_string());

        // forward
        caret.move_to([0, 0].into(), &tx);
        sut.forward(caret);
        assert_eq!(caret, &Caret::new([0, 1].into(), &tx));

        caret.move_to([0, 4].into(), &tx);
        sut.forward(caret);
        assert_eq!(caret, &Caret::new([0, 5].into(), &tx));

        caret.move_to([0, 5].into(), &tx);
        sut.forward(caret);
        assert_eq!(caret, &Caret::new([1, 0].into(), &tx));

        caret.move_to([2, 5].into(), &tx);
        sut.forward(caret);
        assert_eq!(caret, &Caret::new([2, 5].into(), &tx));

        // back
        caret.move_to([0, 3].into(), &tx);
        sut.back(caret);
        assert_eq!(caret, &Caret::new([0, 2].into(), &tx));

        caret.move_to([0, 0].into(), &tx);
        sut.back(caret);
        assert_eq!(caret, &Caret::new([0, 0].into(), &tx));

        caret.move_to([2, 0].into(), &tx);
        sut.back(caret);
        assert_eq!(caret, &Caret::new([1, 6].into(), &tx));

        // previous
        caret.move_to([1, 3].into(), &tx);
        sut.previous(caret);
        assert_eq!(caret, &Caret::new([0, 3].into(), &tx));

        caret.move_to([1, 5].into(), &tx);
        sut.previous(caret);
        assert_eq!(caret, &Caret::new([0, 5].into(), &tx));

        caret.move_to([2, 4].into(), &tx);
        sut.previous(caret);
        assert_eq!(caret, &Caret::new([1, 4].into(), &tx));

        // next
        caret.move_to([0, 3].into(), &tx);
        sut.next(caret);
        assert_eq!(caret, &Caret::new([1, 3].into(), &tx));

        caret.move_to([1, 6].into(), &tx);
        sut.next(caret);
        assert_eq!(caret, &Caret::new([2, 5].into(), &tx));

        caret.move_to([2, 5].into(), &tx);
        sut.next(caret);
        assert_eq!(caret, &Caret::new([2, 5].into(), &tx));
    }

    #[test]
    fn buffer_backspace() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let mut sut = Buffer::new(tx.clone());
        sut.insert_string(
            &mut Caret::new([0, 0].into(), &tx),
            "あいうえお\nかきくけこ\nさしすせそ".to_string(),
        );
        assert_eq!(
            sut.backspace(&mut Caret::new([1, 3].into(), &tx)),
            RemovedChar::Char('く')
        );
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきけこ\nさしすせそ".to_string()
        );
        assert_eq!(
            sut.backspace(&mut Caret::new([1, 4].into(), &tx)),
            RemovedChar::Char('こ')
        );
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきけ\nさしすせそ".to_string()
        );
        assert_eq!(
            sut.backspace(&mut Caret::new([2, 0].into(), &tx)),
            RemovedChar::Enter
        );
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきけさしすせそ".to_string()
        );
    }

    #[test]
    fn buffer_delete() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let mut sut = Buffer::new(tx.clone());
        sut.insert_string(
            &mut Caret::new([0, 0].into(), &tx),
            "あいうえお\nかきくけこ\nさしすせそ".to_string(),
        );
        assert_eq!(
            sut.delete(&Caret::new([1, 3].into(), &tx)),
            RemovedChar::Char('け')
        );
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきくこ\nさしすせそ".to_string()
        );
        assert_eq!(
            sut.delete(&Caret::new([1, 3].into(), &tx)),
            RemovedChar::Char('こ')
        );
        assert_eq!(
            sut.delete(&Caret::new([1, 3].into(), &tx)),
            RemovedChar::Enter
        );
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきくさしすせそ".to_string()
        );
        assert_eq!(
            sut.delete(&Caret::new([1, 7].into(), &tx)),
            RemovedChar::Char('そ')
        );
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきくさしすせ".to_string()
        );
        assert_eq!(
            sut.delete(&Caret::new([1, 7].into(), &tx)),
            RemovedChar::None
        );
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきくさしすせ".to_string()
        );
    }

    #[test]
    fn buffer_line_insert_remove() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let mut sut = BufferLine::default();
        assert_eq!(sut.to_line_string(), "");
        sut.insert_char(0, '鉄', &tx);
        sut.insert_char(1, 'ン', &tx);
        assert_eq!(sut.to_line_string(), "鉄ン");
        sut.insert_char(1, '鍋', &tx);
        sut.insert_char(2, 'の', &tx);
        sut.insert_char(3, 'ャ', &tx);
        sut.insert_char(3, 'ジ', &tx);
        assert_eq!(sut.to_line_string(), "鉄鍋のジャン");
        assert_eq!(sut.remove_char(4, &tx), RemovedChar::Char('ャ'));
        assert_eq!(sut.remove_char(3, &tx), RemovedChar::Char('ジ'));
        assert_eq!(sut.to_line_string(), "鉄鍋のン");
    }

    #[test]
    fn buffer_line_enter_join() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let mut sut = BufferLine::default();
        assert_eq!(sut.to_line_string(), "");
        sut.insert_char(0, '花', &tx);
        sut.insert_char(1, '鳥', &tx);
        sut.insert_char(2, '風', &tx);
        sut.insert_char(3, '月', &tx);
        if let Some(result) = sut.insert_enter(2) {
            assert_eq!(sut.to_line_string(), "花鳥");
            assert_eq!(result.to_line_string(), "風月");
            sut.join(result, &tx);
        } else {
            unreachable!()
        }
        assert_eq!(sut.to_line_string(), "花鳥風月");
        if let Some(result) = sut.insert_enter(4) {
            assert_eq!(sut.to_line_string(), "花鳥風月");
            assert_eq!(result.to_line_string(), "");
        } else {
            unreachable!()
        }
        if sut.insert_enter(5).is_some() {
            unreachable!()
        }
    }

    #[test]
    fn event_test() {
        let (tx, rx) = channel::<ChangeEvent>();
        let mut caret = Caret::new([0, 0].into(), &tx);
        let mut sut = Buffer::new(tx);

        sut.insert_char(&mut caret, 'あ');
        {
            let events: Vec<ChangeEvent> = rx.try_iter().collect();
            #[rustfmt::skip]
            assert_eq!(
                events,
                vec![
                    ChangeEvent::AddCaret(Caret::new_without_event([0, 0].into(), CaretType::Primary)),
                    ChangeEvent::AddChar(BufferChar { position: [0, 0].into(), c: 'あ' }),
                    ChangeEvent::MoveCaret { from: Caret::new_without_event([0, 0].into(), CaretType::Primary), to: Caret::new_without_event([0, 1].into(), CaretType::Primary) }
                ]
            );
        }
        sut.insert_char(&mut caret, 'い');
        {
            let events: Vec<ChangeEvent> = rx.try_iter().collect();
            #[rustfmt::skip]
            assert_eq!(
                events,
                vec![
                    ChangeEvent::AddChar(BufferChar { position: [0, 1].into(), c: 'い' }),
                    ChangeEvent::MoveCaret { from: Caret::new_without_event([0, 1].into(), CaretType::Primary), to: Caret::new_without_event([0, 2].into(), CaretType::Primary) }
                ]
            );
        }
        sut.insert_enter(&mut caret);
        {
            let events: Vec<ChangeEvent> = rx.try_iter().collect();
            #[rustfmt::skip]
            assert_eq!(
                events,
                vec![
                    ChangeEvent::MoveCaret { from: Caret::new_without_event([0, 2].into(), CaretType::Primary), to: Caret::new_without_event([1, 0].into(), CaretType::Primary) }
                ]
            );
            assert_eq!(
                caret,
                Caret::new_without_event([1, 0].into(), CaretType::Primary)
            );
        }
        sut.insert_char(&mut caret, 'え');
        {
            let events: Vec<ChangeEvent> = rx.try_iter().collect();
            #[rustfmt::skip]
            assert_eq!(
                events,
                vec![
                    ChangeEvent::AddChar(BufferChar { position: [1, 0].into(), c: 'え' }),
                    ChangeEvent::MoveCaret { from: Caret::new_without_event([1, 0].into(), CaretType::Primary), to: Caret::new_without_event([1, 1].into(), CaretType::Primary) }
                ]
            );
        }
        sut.insert_char(&mut caret, 'お');
        {
            let events: Vec<ChangeEvent> = rx.try_iter().collect();
            #[rustfmt::skip]
            assert_eq!(
                events,
                vec![
                    ChangeEvent::AddChar(BufferChar { position: [1, 1].into(), c: 'お' }),
                    ChangeEvent::MoveCaret { from: Caret::new_without_event([1, 1].into(), CaretType::Primary), to: Caret::new_without_event([1, 2].into(), CaretType::Primary) }
                ]
            );
        }
    }

    #[test]
    fn event_buffer() {
        let (tx, rx) = channel::<ChangeEvent>();
        let mut caret = Caret::new([0, 0].into(), &tx);
        let mut sut = Buffer::new(tx);

        sut.insert_string(&mut caret, "あいうえお\nかき\nくけ".into());
        sut.buffer_head(&mut caret);
        sut.forward(&mut caret);
        sut.forward(&mut caret);
        let _ = rx.try_iter().collect::<Vec<_>>();
        sut.insert_enter(&mut caret);
        {
            let events: Vec<ChangeEvent> = rx.try_iter().collect();
            #[rustfmt::skip]
            assert_eq!(
                events,
                vec![
                    ChangeEvent::MoveChar { from: BufferChar { position: [2, 0].into(), c: 'く' }, to: BufferChar { position: [3, 0].into(), c: 'く' } },
                    ChangeEvent::MoveChar { from: BufferChar { position: [2, 1].into(), c: 'け' }, to: BufferChar { position: [3, 1].into(), c: 'け' } },
                    ChangeEvent::MoveChar { from: BufferChar { position: [1, 0].into(), c: 'か' }, to: BufferChar { position: [2, 0].into(), c: 'か' } },
                    ChangeEvent::MoveChar { from: BufferChar { position: [1, 1].into(), c: 'き' }, to: BufferChar { position: [2, 1].into(), c: 'き' } },
                    // 先に以降の行を逆順で移動してから、改行対象の行を動かす
                    ChangeEvent::MoveChar { from: BufferChar { position: [0, 2].into(), c: 'う' }, to: BufferChar { position: [1, 0].into(), c: 'う' } },
                    ChangeEvent::MoveChar { from: BufferChar { position: [0, 3].into(), c: 'え' }, to: BufferChar { position: [1, 1].into(), c: 'え' } },
                    ChangeEvent::MoveChar { from: BufferChar { position: [0, 4].into(), c: 'お' }, to: BufferChar { position: [1, 2].into(), c: 'お' } },
                    ChangeEvent::MoveCaret { from: Caret::new_without_event([0, 2].into(), CaretType::Primary), to: Caret::new_without_event([1, 0].into(), CaretType::Primary) },
                ]
            );
        }
    }

    #[test]
    fn event_line_add() {
        let (tx, rx) = channel::<ChangeEvent>();
        let mut caret = Caret::new([0, 0].into(), &tx);
        let mut sut = Buffer::new(tx);

        sut.insert_string(&mut caret, "あいうえお".into());
        sut.head(&mut caret);
        sut.forward(&mut caret);
        let _ = rx.try_iter().collect::<Vec<_>>();
        sut.insert_char(&mut caret, 'A');
        {
            let events: Vec<ChangeEvent> = rx.try_iter().collect();
            #[rustfmt::skip]
            assert_eq!(
                events,
                vec![
                    ChangeEvent::MoveChar { from: BufferChar { position: [0, 4].into(), c: 'お' }, to: BufferChar { position: [0, 5].into(), c: 'お' } },
                    ChangeEvent::MoveChar { from: BufferChar { position: [0, 3].into(), c: 'え' }, to: BufferChar { position: [0, 4].into(), c: 'え' } },
                    ChangeEvent::MoveChar { from: BufferChar { position: [0, 2].into(), c: 'う' }, to: BufferChar { position: [0, 3].into(), c: 'う' } },
                    ChangeEvent::MoveChar { from: BufferChar { position: [0, 1].into(), c: 'い' }, to: BufferChar { position: [0, 2].into(), c: 'い' } },
                    ChangeEvent::AddChar(BufferChar { position: [0, 1].into(), c: 'A' }),
                    ChangeEvent::MoveCaret { from: Caret::new_without_event([0, 1].into(), CaretType::Primary), to: Caret::new_without_event([0, 2].into(), CaretType::Primary) },
                ]
            );
        }
    }

    #[test]
    fn event_line_delete() {
        let (tx, rx) = channel::<ChangeEvent>();
        let mut caret = Caret::new([0, 0].into(), &tx);
        let mut sut = Buffer::new(tx);

        sut.insert_string(&mut caret, "あいうえお".into());
        sut.back(&mut caret);
        sut.back(&mut caret);
        let _ = rx.try_iter().collect::<Vec<_>>();
        sut.backspace(&mut caret);
        {
            let events: Vec<ChangeEvent> = rx.try_iter().collect();
            #[rustfmt::skip]
            assert_eq!(
                events,
                vec![
                    ChangeEvent::MoveCaret { from: Caret::new_without_event([0, 3].into(), CaretType::Primary), to: Caret::new_without_event([0, 2].into(), CaretType::Primary)},
                    ChangeEvent::RemoveChar(BufferChar { position: [0, 2].into(), c: 'う' }),
                    ChangeEvent::MoveChar { from: BufferChar { position: [0, 3].into(), c: 'え' }, to: BufferChar { position: [0, 2].into(), c: 'え' } },
                    ChangeEvent::MoveChar { from: BufferChar { position: [0, 4].into(), c: 'お' }, to: BufferChar { position: [0, 3].into(), c: 'お' } },
                ]
            );
        }
    }

    #[test]
    fn buffer_copy() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let mut sut = Buffer::new(tx.clone());
        sut.insert_string(
            &mut Caret::new([0, 0].into(), &tx),
            "あいうえお\nかきくけこ\nさしすせそそ".to_string(),
        );
        {
            // Caret が隣接する時には一文字だけ
            assert_eq!(
                sut.copy_string(
                    &Caret::new([0, 1].into(), &tx),
                    &Caret::new([0, 2].into(), &tx)
                ),
                "い"
            );
            assert_eq!(
                sut.copy_string(
                    &Caret::new([0, 2].into(), &tx),
                    &Caret::new([0, 1].into(), &tx)
                ),
                "い"
            );
        }
        {
            // 複数行
            assert_eq!(
                sut.copy_string(
                    &Caret::new([1, 2].into(), &tx),
                    &Caret::new([2, 3].into(), &tx)
                ),
                "くけこ\nさしす"
            );
            assert_eq!(
                sut.copy_string(
                    &Caret::new([0, 4].into(), &tx),
                    &Caret::new([2, 3].into(), &tx)
                ),
                "お\nかきくけこ\nさしす"
            );
            // Caret の位置によっては前後に改行を取ってくる動きをする
            assert_eq!(
                sut.copy_string(
                    &Caret::new([0, 5].into(), &tx),
                    &Caret::new([2, 0].into(), &tx)
                ),
                "\nかきくけこ\n"
            );
        }
    }

    #[test]
    fn buffer_char_in_caret_range() {
        let (tx, _rx) = channel::<ChangeEvent>();

        struct Case {
            from: Caret,
            to: Caret,
            target: BufferChar,
            expected: bool,
        }
        let cases = vec![
            Case {
                from: Caret::new([0, 0].into(), &tx),
                to: Caret::new([0, 10].into(), &tx),
                target: BufferChar {
                    position: [0, 5].into(),
                    c: 'あ',
                },
                expected: true,
            },
            Case {
                from: Caret::new([0, 0].into(), &tx),
                to: Caret::new([2, 0].into(), &tx),
                target: BufferChar {
                    position: [1, 5].into(),
                    c: 'あ',
                },
                expected: true,
            },
            Case {
                from: Caret::new([0, 0].into(), &tx),
                to: Caret::new([0, 5].into(), &tx),
                target: BufferChar {
                    position: [1, 5].into(),
                    c: 'あ',
                },
                expected: false,
            },
            Case {
                from: Caret::new([0, 0].into(), &tx),
                to: Caret::new([0, 4].into(), &tx),
                target: BufferChar {
                    position: [0, 5].into(),
                    c: 'あ',
                },
                expected: false,
            },
            Case {
                from: Caret::new([0, 0].into(), &tx),
                to: Caret::new([0, 4].into(), &tx),
                target: BufferChar {
                    position: [0, 4].into(),
                    c: 'あ',
                },
                expected: false,
            },
        ];
        for c in cases {
            assert_eq!(c.target.in_caret_range(c.from, c.to), c.expected);
        }
    }

    #[test]
    fn test_highlight_positions() {
        struct TestCase {
            test_string: &'static str,
            higlight_string: &'static str,
            cell_positions: Vec<CellPosition>,
        }
        let cases = vec![
            TestCase {
                test_string: "Hello, World!",
                higlight_string: "World",
                cell_positions: vec![CellPosition { row: 0, col: 7 }],
            },
            TestCase {
                test_string: "Hello, World! World!",
                higlight_string: "World",
                cell_positions: vec![
                    CellPosition { row: 0, col: 7 },
                    CellPosition { row: 0, col: 14 },
                ],
            },
            TestCase {
                test_string: indoc::indoc! {r#"
                    Hello,
                    Good World!
                    And
                    Bad World.
                "#},
                higlight_string: "World",
                cell_positions: vec![
                    CellPosition { row: 1, col: 5 },
                    CellPosition { row: 3, col: 4 },
                ],
            },
            TestCase {
                test_string: indoc::indoc! {r#"
                    Hello!
                    Good Bye.
                "#},
                higlight_string: "Hi!",
                cell_positions: vec![],
            },
            TestCase {
                test_string: "こんにちは、世界！🐖世界！",
                higlight_string: "世界",
                cell_positions: vec![
                    CellPosition { row: 0, col: 6 },
                    CellPosition { row: 0, col: 10 },
                ],
            },
        ];
        for case in cases.iter() {
            let (sender, receiver) = std::sync::mpsc::channel();
            let mut sut = Buffer::new(sender.clone());
            let mut caret = Caret::new([0, 0].into(), &sender);
            sut.insert_string(&mut caret, case.test_string.to_string());
            let _ = receiver.try_iter().collect::<Vec<_>>();

            let result = sut.highlight_positions(case.higlight_string);
            assert_eq!(result, case.cell_positions);
        }
    }

    #[test]
    fn test_highlight() {
        struct TestCase {
            test_string: &'static str,
            higlight_string: &'static str,
            events: Vec<ChangeEvent>,
        }
        let cases = vec![TestCase {
            test_string: "Hello, World!",
            higlight_string: "World",
            events: vec![
                ChangeEvent::SelectChar(BufferChar {
                    position: [0, 7].into(),
                    c: 'W',
                }),
                ChangeEvent::SelectChar(BufferChar {
                    position: [0, 8].into(),
                    c: 'o',
                }),
                ChangeEvent::SelectChar(BufferChar {
                    position: [0, 9].into(),
                    c: 'r',
                }),
                ChangeEvent::SelectChar(BufferChar {
                    position: [0, 10].into(),
                    c: 'l',
                }),
                ChangeEvent::SelectChar(BufferChar {
                    position: [0, 11].into(),
                    c: 'd',
                }),
            ],
        }];
        for case in cases.into_iter() {
            let (sender, receiver) = std::sync::mpsc::channel();
            let mut sut = Buffer::new(sender.clone());
            let mut caret = Caret::new([0, 0].into(), &sender);
            sut.insert_string(&mut caret, case.test_string.to_string());
            let _ = receiver.try_iter().collect::<Vec<_>>();

            sut.highlight(case.higlight_string);
            for event in case.events.into_iter() {
                assert_eq!(receiver.recv(), Ok(event));
            }
        }
    }

    #[test]
    fn test_move_to_next() {
        struct TestCase {
            test_string: &'static str,
            higlight_string: &'static str,
            start_position: CellPosition,
            moved_positions: Vec<CellPosition>,
        }
        let cases = [
            TestCase {
                test_string: indoc::indoc! {r#"
                    My name is nes.
                    Friend is pola.
                    Other friend Is roid.
                "#},
                higlight_string: "is",
                start_position: CellPosition { row: 0, col: 8 },
                moved_positions: vec![
                    CellPosition { row: 1, col: 7 },
                    CellPosition { row: 0, col: 8 },
                    CellPosition { row: 1, col: 7 },
                ],
            },
            TestCase {
                test_string: indoc::indoc! {r#"
                    本日はお日柄もよく。
                    銀行員もよく笑っております。
                    くよくよしなさんな。
                "#},
                higlight_string: "よく",
                start_position: CellPosition { row: 0, col: 0 },
                moved_positions: vec![
                    CellPosition { row: 0, col: 7 },
                    CellPosition { row: 1, col: 4 },
                    CellPosition { row: 2, col: 1 },
                ],
            },
        ];
        for case in cases.iter() {
            let (sender, receiver) = std::sync::mpsc::channel();
            let mut sut = Buffer::new(sender.clone());
            let mut caret = Caret::new([0, 0].into(), &sender);
            sut.insert_string(&mut caret, case.test_string.to_string());
            let _ = receiver.try_iter().collect::<Vec<_>>();

            let mut caret = Caret::new(case.start_position, &sender);

            case.moved_positions.iter().for_each(|p| {
                sut.move_to_next(&mut caret, case.higlight_string);
                assert_eq!(caret.position, *p);
            });
        }
    }

    #[test]
    fn test_move_to_previous() {
        struct TestCase {
            test_string: &'static str,
            higlight_string: &'static str,
            start_position: CellPosition,
            moved_positions: Vec<CellPosition>,
        }
        let cases = [
            TestCase {
                test_string: indoc::indoc! {r#"
                    My name is nes.
                    Friend is pola.
                    Other friend Is roid.
                "#},
                higlight_string: "is",
                start_position: CellPosition { row: 1, col: 0 },
                moved_positions: vec![
                    CellPosition { row: 0, col: 8 },
                    CellPosition { row: 1, col: 7 },
                    CellPosition { row: 0, col: 8 },
                ],
            },
            TestCase {
                test_string: indoc::indoc! {r#"
                    本日はお日柄もよく。
                    銀行員もよく笑っております。
                    くよくよしなさんな。
                "#},
                higlight_string: "よく",
                start_position: CellPosition { row: 2, col: 8 },
                moved_positions: vec![
                    CellPosition { row: 2, col: 1 },
                    CellPosition { row: 1, col: 4 },
                    CellPosition { row: 0, col: 7 },
                    CellPosition { row: 2, col: 1 },
                ],
            },
        ];
        for case in cases.iter() {
            let (sender, receiver) = std::sync::mpsc::channel();
            let mut sut = Buffer::new(sender.clone());
            let mut caret = Caret::new([0, 0].into(), &sender);
            sut.insert_string(&mut caret, case.test_string.to_string());
            let _ = receiver.try_iter().collect::<Vec<_>>();

            let mut caret = Caret::new(case.start_position, &sender);

            case.moved_positions.iter().for_each(|p| {
                sut.move_to_previous(&mut caret, case.higlight_string);
                assert_eq!(caret.position, *p);
            });
        }
    }
}
