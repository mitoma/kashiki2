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
    row_num: i64,
    chars: Vec<BufferChar>,
}

impl BufferLine {
    pub fn new() -> BufferLine {
        BufferLine {
            row_num: 0,
            chars: Vec::new(),
        }
    }

    pub fn update_row_num(&mut self, row_num: i64) {
        self.row_num = row_num;
        (0_i64..).zip(self.chars.iter_mut()).for_each(|(i, c)| {
            c.update_position(row_num, i);
        })
    }

    pub fn pos_string(&self, position: usize) -> &str {
        &self.chars[position].c
    }
}

pub struct BufferChar {
    row: i64,
    col: i64,
    c: String,
}

impl BufferChar {
    pub fn new(row: i64, col: i64, c: String) -> BufferChar {
        BufferChar {
            row: row,
            col: col,
            c: c,
        }
    }

    pub fn update_position(&mut self, row: i64, col: i64) {
        self.row = row;
        self.col = col;
    }
}
