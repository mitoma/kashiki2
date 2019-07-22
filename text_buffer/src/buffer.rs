pub struct Caret {
    row: usize,
    col: usize,
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

    pub fn insert_char(&mut self, mut caret: Caret, c: char) {
        match self.lines.get_mut(caret.row) {
            Some(line) => {
                line.insert_char(caret.col, c);
                caret.col +=1
            }
            None => {}
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

    pub fn insert_enter(&mut self, col: usize) -> BufferLine {
        BufferLine::from_chars(self.chars.split_off(col))
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
        let mut sut = Buffer::new("hello buffer".to_string());
        assert_eq!(sut.to_buffer_string(), "");
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
        let sut2 = sut.insert_enter(2);
        assert_eq!(sut.to_line_string(), "花鳥");
        assert_eq!(sut2.to_line_string(), "風月");
        sut.join(sut2);
        assert_eq!(sut.to_line_string(), "花鳥風月");
    }

}
