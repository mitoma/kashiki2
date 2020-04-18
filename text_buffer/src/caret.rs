#[derive(Debug, PartialEq, Clone)]
pub struct Caret {
    pub row: usize,
    pub col: usize,
}

impl Caret {
    pub fn new(row: usize, col: usize) -> Caret {
        Caret { row: row, col: col }
    }

    pub fn move_to(&mut self, row: usize, col: usize) {
        self.row = row;
        self.col = col;
    }
}
