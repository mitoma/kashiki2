#[derive(Debug, PartialEq, Clone)]
pub struct Caret {
    pub row: usize,
    pub col: usize,
}

impl Caret {
    pub fn new(row: usize, col: usize) -> Caret {
        Caret { row, col }
    }

    pub fn move_to(&mut self, row: usize, col: usize) {
        self.row = row;
        self.col = col;
    }

    pub fn to(&mut self, to: &Caret) {
        self.row = to.row;
        self.col = to.col;
    }
}
