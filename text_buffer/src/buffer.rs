pub struct Caret {
    row: usize,
    col: usize,
}

impl Caret {
    pub fn new(row: usize, col: usize) -> Caret {
        Caret { row: row, col: col }
    }
}

pub struct Buffer {
    buffer_name: String,
    lines: Vec<BufferLine>,
}

impl Buffer {
    pub fn new(buffer_name: String) -> Buffer {
        let mut lines = Vec::new();
        let line = BufferLine::new();
        lines.push(line);
        Buffer {
            buffer_name: buffer_name,
            lines: lines,
        }
    }

    pub fn to_buffer_string(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.to_line_string())
            .collect::<Vec<String>>()
            .join("\n")
    }

    pub fn insert_string(&mut self, caret: Caret, string: String) -> Caret {
        let mut iter = string.split("\r\n").flat_map(|line| line.split('\n'));
        let first_line = match iter.next() {
            Some(first) => first,
            None => return caret,
        };
        let caret = first_line
            .chars()
            .fold(caret, |caret, c| self.insert_char(caret, c));
        iter.fold(caret, |caret, line| {
            let caret = self.insert_enter(caret);
            line.chars()
                .fold(caret, |caret, c| self.insert_char(caret, c))
        })
    }

    pub fn insert_char(&mut self, mut caret: Caret, c: char) -> Caret {
        if let Some(line) = self.lines.get_mut(caret.row) {
            line.insert_char(caret.col, c);
            caret.col += 1;
        }
        caret
    }

    pub fn insert_enter(&mut self, mut caret: Caret) -> Caret {
        if let Some(line) = self.lines.get_mut(caret.row) {
            if let Some(next_line) = line.insert_enter(caret.col) {
                caret.row += 1;
                caret.col = 0;
                self.lines.insert(caret.row, next_line);
                self.update_position();
            }
        }
        caret
    }

    pub fn update_position(&mut self) {
        (0..).zip(self.lines.iter_mut()).for_each(|(i, l)| {
            l.update_position(i);
        })
    }

    pub fn head(&mut self, mut caret: Caret) -> Caret {
        caret.col = 0;
        caret
    }

    pub fn last(&mut self, mut caret: Caret) -> Caret {
        if let Some(line) = self.lines.get(caret.row) {
            caret.col = line.chars.len();
        }
        caret
    }

    pub fn back(&mut self, mut caret: Caret) -> Caret {
        if self.is_line_head(&caret) {
            if self.is_buffer_head(&caret) {
                caret
            } else {
                let caret = self.previous(caret);
                self.last(caret)
            }
        } else {
            caret.col -= 1;
            caret
        }
    }

    pub fn forward(&mut self, mut caret: Caret) -> Caret {
        if self.is_line_last(&caret) {
            if self.is_buffer_last(&caret) {
                caret
            } else {
                let caret = self.next(caret);
                self.head(caret)
            }
        } else {
            caret.col += 1;
            caret
        }
    }

    pub fn previous(&mut self, mut caret: Caret) -> Caret {
        if self.is_buffer_head(&caret) {
            caret
        } else {
            caret.row -= 1;
            if caret.col > self.lines.get(caret.row).unwrap().chars.len() {
                self.last(caret)
            } else {
                caret
            }
        }
    }

    pub fn next(&mut self, mut caret: Caret) -> Caret {
        if self.is_buffer_last(&caret) {
            caret
        } else {
            caret.row += 1;
            if caret.col > self.lines.get(caret.row).unwrap().chars.len() {
                self.last(caret)
            } else {
                caret
            }
        }
    }

    pub fn buffer_head(&mut self, mut caret: Caret) -> Caret {
        caret.row = 0;
        caret.col = 0;
        caret
    }

    pub fn buffer_last(&mut self, mut caret: Caret) -> Caret {
        if let Some(last_line) = self.lines.last() {
            caret.row = last_line.row_num;
            caret.col = last_line.chars.len();
        }
        caret
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
            caret.col == line_length - 1
        } else {
            false
        }
    }
}

pub struct BufferLine {
    row_num: usize,
    chars: Vec<BufferChar>,
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

    pub fn remove_char(&mut self, col: usize) {
        self.chars.remove(col);
    }

    pub fn join(&mut self, line: BufferLine) {
        line.chars.into_iter().for_each({ |c| self.chars.push(c) })
    }
}

pub struct BufferChar {
    row: usize,
    col: usize,
    c: char,
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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn buffer() {
        let caret = Caret::new(0, 0);
        let mut sut = Buffer::new("hello buffer".to_string());
        assert_eq!(sut.to_buffer_string(), "");
        let caret = sut.insert_char(caret, '山');
        assert_eq!(sut.to_buffer_string(), "山");
        assert_eq!(caret.row, 0);
        assert_eq!(caret.col, 1);
        let caret = sut.insert_char(caret, '本');
        assert_eq!(sut.to_buffer_string(), "山本");
        assert_eq!(caret.row, 0);
        assert_eq!(caret.col, 2);
        let caret = sut.insert_enter(caret);
        assert_eq!(sut.to_buffer_string(), "山本\n");
        assert_eq!(caret.row, 1);
        assert_eq!(caret.col, 0);
        let caret = sut.insert_enter(caret);
        assert_eq!(sut.to_buffer_string(), "山本\n\n");
        assert_eq!(caret.row, 2);
        assert_eq!(caret.col, 0);
        sut.insert_enter(Caret::new(100, 100));
        assert_eq!(sut.to_buffer_string(), "山本\n\n");
    }

    #[test]
    fn buffer_insert_string() {
        let caret = Caret::new(0, 0);
        let mut sut = Buffer::new("hello buffer".to_string());
        let caret = sut.insert_string(
            caret,
            "東京は\n今日もいい天気\nだった。".to_string(),
        );
        assert_eq!(
            sut.to_buffer_string(),
            "東京は\n今日もいい天気\nだった。"
        );
        assert_eq!(caret.row, 2);
        assert_eq!(caret.col, 4);
    }

    #[test]
    fn buffer_position_check() {
        let mut sut = Buffer::new("hello buffer".to_string());
        let _caret = sut.insert_string(
            Caret::new(0, 0),
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
        assert!(sut.is_line_last(&Caret::new(0, 4)));
        assert!(sut.is_line_last(&Caret::new(2, 5)));
        assert!(!sut.is_line_last(&Caret::new(2, 4)));
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
        sut.remove_char(4);
        sut.remove_char(3);
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
