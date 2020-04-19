use crate::caret::Caret;

pub struct Buffer {
    pub lines: Vec<BufferLine>,
}

impl Buffer {
    pub fn new() -> Buffer {
        let mut lines = Vec::new();
        let line = BufferLine::new();
        lines.push(line);
        Buffer { lines: lines }
    }

    pub fn to_buffer_string(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.to_line_string())
            .collect::<Vec<String>>()
            .join("\n")
    }

    pub fn insert_string(&mut self, caret: &mut Caret, string: String) {
        let mut iter = string.split("\r\n").flat_map(|line| line.split('\n'));
        let first_line = match iter.next() {
            Some(first) => first,
            None => return,
        };
        first_line.chars().for_each(|c| self.insert_char(caret, c));
        iter.for_each(|line| {
            self.insert_enter(caret);
            line.chars().for_each(|c| self.insert_char(caret, c))
        })
    }

    pub fn insert_char(&mut self, caret: &mut Caret, c: char) {
        if let Some(line) = self.lines.get_mut(caret.row) {
            line.insert_char(caret.col, c);
            caret.col += 1;
        }
    }

    pub fn insert_enter(&mut self, caret: &mut Caret) {
        if let Some(line) = self.lines.get_mut(caret.row) {
            if let Some(next_line) = line.insert_enter(caret.col) {
                caret.row += 1;
                caret.col = 0;
                self.lines.insert(caret.row, next_line);
                self.update_position();
            }
        }
    }

    pub fn update_position(&mut self) {
        (0..).zip(self.lines.iter_mut()).for_each(|(i, l)| {
            l.update_position(i);
        })
    }

    pub fn head(&mut self, caret: &mut Caret) {
        caret.col = 0;
    }

    pub fn last(&mut self, caret: &mut Caret) {
        if let Some(line) = self.lines.get(caret.row) {
            caret.col = line.chars.len();
        }
    }

    pub fn back(&mut self, caret: &mut Caret) {
        match (self.is_line_head(&caret), self.is_buffer_head(&caret)) {
            (true, true) => {}
            (true, false) => {
                self.previous(caret);
                self.last(caret);
            }
            (false, true) | (false, false) => caret.col -= 1,
        }
    }

    pub fn forward(&mut self, caret: &mut Caret) {
        match (self.is_line_last(&caret), self.is_buffer_last(&caret)) {
            (true, true) => {}
            (true, false) => {
                self.next(caret);
                self.head(caret);
            }
            (false, true) | (false, false) => caret.col += 1,
        }
    }

    pub fn previous(&mut self, caret: &mut Caret) {
        if self.is_buffer_head(&caret) {
            return;
        } else {
            caret.row -= 1;
            if self.is_line_last(caret) {
                self.last(caret)
            } else {
                return;
            }
        }
    }

    pub fn next(&mut self, caret: &mut Caret) {
        if self.is_buffer_last(&caret) {
            return;
        } else {
            caret.row += 1;
            if self.is_line_last(caret) {
                self.last(caret)
            } else {
                return;
            }
        }
    }

    pub fn buffer_head(&mut self, caret: &mut Caret) {
        caret.row = 0;
        caret.col = 0;
    }

    pub fn buffer_last(&mut self, caret: &mut Caret) {
        if let Some(last_line) = self.lines.last() {
            caret.row = last_line.row_num;
            caret.col = last_line.chars.len();
        }
    }

    pub fn is_buffer_head(&mut self, caret: &Caret) -> bool {
        caret.row == 0
    }

    pub fn is_buffer_last(&mut self, caret: &Caret) -> bool {
        caret.row == self.lines.len() - 1
    }

    pub fn is_line_head(&mut self, caret: &Caret) -> bool {
        caret.col == 0
    }

    pub fn is_line_last(&mut self, caret: &Caret) -> bool {
        if let Some(line_length) = self.lines.get(caret.row).map(|line| line.chars.len()) {
            caret.col >= line_length
        } else {
            false
        }
    }

    pub fn backspace(&mut self, caret: &mut Caret) -> RemovedChar {
        if self.is_buffer_head(&caret) && self.is_line_head(&caret) {
            RemovedChar::None
        } else {
            self.back(caret);
            let removed_char = self.delete(caret);
            removed_char
        }
    }

    pub fn delete(&mut self, caret: &Caret) -> RemovedChar {
        if self.is_line_last(&caret) {
            if !self.is_buffer_last(&caret) {
                let next_line = self.lines.remove(caret.row + 1);
                let current_line = self.lines.get_mut(caret.row).unwrap();
                current_line.join(next_line);
                RemovedChar::Enter
            } else {
                RemovedChar::None
            }
        } else {
            if let Some(line) = self.lines.get_mut(caret.row) {
                line.remove_char(caret.col)
            } else {
                RemovedChar::None
            }
        }
    }
}

pub struct BufferLine {
    row_num: usize,
    pub chars: Vec<BufferChar>,
}

impl BufferLine {
    pub fn new() -> BufferLine {
        BufferLine {
            row_num: 0,
            chars: Vec::new(),
        }
    }

    pub fn from_chars(chars: Vec<BufferChar>) -> BufferLine {
        BufferLine {
            row_num: 0,
            chars: chars,
        }
    }

    pub fn update_position(&mut self, row_num: usize) {
        self.row_num = row_num;
        (0..).zip(self.chars.iter_mut()).for_each(|(i, c)| {
            c.update_position(row_num, i);
        })
    }

    pub fn to_line_string(&self) -> String {
        self.chars.iter().map(|c| c.c).collect()
    }

    pub fn pos_char(&self, position: usize) -> char {
        self.chars[position].c
    }

    pub fn insert_char(&mut self, col: usize, c: char) {
        self.chars
            .insert(col, BufferChar::new(col, self.row_num, c))
    }

    pub fn insert_enter(&mut self, col: usize) -> Option<BufferLine> {
        if self.chars.len() == col {
            Some(BufferLine::new())
        } else if self.chars.len() > col {
            Some(BufferLine::from_chars(self.chars.split_off(col)))
        } else {
            None
        }
    }

    pub fn remove_char(&mut self, col: usize) -> RemovedChar {
        let removed = self.chars.remove(col);
        RemovedChar::Char(removed.c)
    }

    pub fn join(&mut self, line: BufferLine) {
        line.chars.into_iter().for_each({ |c| self.chars.push(c) })
    }
}

pub struct BufferChar {
    pub row: usize,
    pub col: usize,
    pub c: char,
}

impl BufferChar {
    pub fn new(row: usize, col: usize, c: char) -> BufferChar {
        BufferChar {
            row: row,
            col: col,
            c: c,
        }
    }

    pub fn update_position(&mut self, row: usize, col: usize) {
        self.row = row;
        self.col = col;
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

    use super::*;

    #[test]
    fn buffer() {
        let caret = &mut Caret::new(0, 0);
        let mut sut = Buffer::new();
        assert_eq!(sut.to_buffer_string(), "");
        sut.insert_char(caret, '山');
        assert_eq!(sut.to_buffer_string(), "山");
        assert_eq!(caret.row, 0);
        assert_eq!(caret.col, 1);
        sut.insert_char(caret, '本');
        assert_eq!(sut.to_buffer_string(), "山本");
        assert_eq!(caret.row, 0);
        assert_eq!(caret.col, 2);
        sut.insert_enter(caret);
        assert_eq!(sut.to_buffer_string(), "山本\n");
        assert_eq!(caret.row, 1);
        assert_eq!(caret.col, 0);
        sut.insert_enter(caret);
        assert_eq!(sut.to_buffer_string(), "山本\n\n");
        assert_eq!(caret.row, 2);
        assert_eq!(caret.col, 0);
        sut.insert_enter(&mut Caret::new(100, 100));
        assert_eq!(sut.to_buffer_string(), "山本\n\n");
    }

    #[test]
    fn buffer_insert_string() {
        let caret = &mut Caret::new(0, 0);
        let mut sut = Buffer::new();
        sut.insert_string(caret, "東京は\n今日もいい天気\nだった。".to_string());
        assert_eq!(sut.to_buffer_string(), "東京は\n今日もいい天気\nだった。");
        assert_eq!(caret.row, 2);
        assert_eq!(caret.col, 4);
    }

    #[test]
    fn buffer_position_check() {
        let mut sut = Buffer::new();
        sut.insert_string(
            &mut Caret::new(0, 0),
            "あいうえお\nかきくけこ\nさしすせそそ".to_string(),
        );
        // buffer head
        assert!(sut.is_buffer_head(&Caret::new(0, 0)));
        assert!(sut.is_buffer_head(&Caret::new(0, 4)));
        assert!(!sut.is_buffer_head(&Caret::new(1, 0)));

        // buffer last
        assert!(sut.is_buffer_last(&Caret::new(2, 0)));
        assert!(sut.is_buffer_last(&Caret::new(2, 4)));
        assert!(!sut.is_buffer_last(&Caret::new(0, 0)));

        // line head
        assert!(sut.is_line_head(&Caret::new(0, 0)));
        assert!(sut.is_line_head(&Caret::new(2, 0)));
        assert!(!sut.is_line_head(&Caret::new(1, 3)));

        // line last
        assert!(sut.is_line_last(&Caret::new(0, 5)));
        assert!(sut.is_line_last(&Caret::new(2, 6)));
        assert!(!sut.is_line_last(&Caret::new(2, 5)));
    }

    #[test]
    fn buffer_move() {
        let mut sut = Buffer::new();
        let caret = &mut Caret::new(0, 0);
        let _caret = sut.insert_string(caret, "あいうえお\nきかくけここ\nさしすせそ".to_string());

        // forward
        caret.move_to(0, 0);
        sut.forward(caret);
        assert_eq!(caret, &Caret::new(0, 1));

        caret.move_to(0, 4);
        sut.forward(caret);
        assert_eq!(caret, &Caret::new(0, 5));

        caret.move_to(0, 5);
        sut.forward(caret);
        assert_eq!(caret, &Caret::new(1, 0));

        caret.move_to(2, 5);
        sut.forward(caret);
        assert_eq!(caret, &Caret::new(2, 5));

        // back
        caret.move_to(0, 3);
        sut.back(caret);
        assert_eq!(caret, &Caret::new(0, 2));

        caret.move_to(0, 0);
        sut.back(caret);
        assert_eq!(caret, &Caret::new(0, 0));

        caret.move_to(2, 0);
        sut.back(caret);
        assert_eq!(caret, &Caret::new(1, 6));

        // previous
        caret.move_to(1, 3);
        sut.previous(caret);
        assert_eq!(caret, &Caret::new(0, 3));

        caret.move_to(1, 5);
        sut.previous(caret);
        assert_eq!(caret, &Caret::new(0, 5));

        caret.move_to(2, 4);
        sut.previous(caret);
        assert_eq!(caret, &Caret::new(1, 4));

        // next
        caret.move_to(0, 3);
        sut.next(caret);
        assert_eq!(caret, &Caret::new(1, 3));

        caret.move_to(1, 6);
        sut.next(caret);
        assert_eq!(caret, &Caret::new(2, 5));

        caret.move_to(2, 5);
        sut.next(caret);
        assert_eq!(caret, &Caret::new(2, 5));
    }

    #[test]
    fn buffer_backspace() {
        let mut sut = Buffer::new();
        let _caret = sut.insert_string(
            &mut Caret::new(0, 0),
            "あいうえお\nかきくけこ\nさしすせそ".to_string(),
        );
        assert_eq!(
            sut.backspace(&mut Caret::new(1, 3)),
            RemovedChar::Char('く')
        );
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきけこ\nさしすせそ".to_string()
        );
        assert_eq!(
            sut.backspace(&mut Caret::new(1, 4)),
            RemovedChar::Char('こ')
        );
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきけ\nさしすせそ".to_string()
        );
        assert_eq!(sut.backspace(&mut Caret::new(2, 0)), RemovedChar::Enter);
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきけさしすせそ".to_string()
        );
    }

    #[test]
    fn buffer_delete() {
        let mut sut = Buffer::new();
        let _caret = sut.insert_string(
            &mut Caret::new(0, 0),
            "あいうえお\nかきくけこ\nさしすせそ".to_string(),
        );
        assert_eq!(sut.delete(&Caret::new(1, 3)), RemovedChar::Char('け'));
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきくこ\nさしすせそ".to_string()
        );
        assert_eq!(sut.delete(&Caret::new(1, 3)), RemovedChar::Char('こ'));
        assert_eq!(sut.delete(&Caret::new(1, 3)), RemovedChar::Enter);
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきくさしすせそ".to_string()
        );
        assert_eq!(sut.delete(&Caret::new(1, 7)), RemovedChar::Char('そ'));
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきくさしすせ".to_string()
        );
        assert_eq!(sut.delete(&Caret::new(1, 7)), RemovedChar::None);
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきくさしすせ".to_string()
        );
    }

    #[test]
    fn buffer_line_insert_remove() {
        let mut sut = BufferLine::new();
        assert_eq!(sut.to_line_string(), "");
        sut.insert_char(0, '鉄');
        sut.insert_char(1, 'ン');
        assert_eq!(sut.to_line_string(), "鉄ン");
        sut.insert_char(1, '鍋');
        sut.insert_char(2, 'の');
        sut.insert_char(3, 'ャ');
        sut.insert_char(3, 'ジ');
        assert_eq!(sut.to_line_string(), "鉄鍋のジャン");
        assert_eq!(sut.remove_char(4), RemovedChar::Char('ャ'));
        assert_eq!(sut.remove_char(3), RemovedChar::Char('ジ'));
        assert_eq!(sut.to_line_string(), "鉄鍋のン");
    }

    #[test]
    fn buffer_line_enter_join() {
        let mut sut = BufferLine::new();
        assert_eq!(sut.to_line_string(), "");
        sut.insert_char(0, '花');
        sut.insert_char(1, '鳥');
        sut.insert_char(2, '風');
        sut.insert_char(3, '月');
        if let Some(result) = sut.insert_enter(2) {
            assert_eq!(sut.to_line_string(), "花鳥");
            assert_eq!(result.to_line_string(), "風月");
            sut.join(result);
        } else {
            assert!(false);
        }
        assert_eq!(sut.to_line_string(), "花鳥風月");
        if let Some(result) = sut.insert_enter(4) {
            assert_eq!(sut.to_line_string(), "花鳥風月");
            assert_eq!(result.to_line_string(), "");
        } else {
            assert!(false);
        }
        if let Some(_) = sut.insert_enter(5) {
            assert!(false);
        } else {
            assert!(true);
        }
    }
}
