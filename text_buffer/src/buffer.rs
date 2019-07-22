pub struct Buffer {
    buffer_name: String,
    lines: Vec<BufferLine>,
}

impl Buffer {
    pub fn new(buffer_name: String) -> Buffer {
        Buffer {
            buffer_name: buffer_name,
            lines: Vec::new(),
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

    pub fn remove_char(&mut self, col: usize) {
        self.chars.remove(col);
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
    fn buffer_line() {
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
}
