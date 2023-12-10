use std::sync::mpsc::Sender;

use crate::editor::ChangeEvent;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Caret {
    pub row: usize,
    pub col: usize,
}

impl Caret {
    pub fn new(row: usize, col: usize, sender: &Sender<ChangeEvent>) -> Self {
        let instance = Self { row, col };
        sender.send(ChangeEvent::AddCaret(instance)).unwrap();
        instance
    }

    pub fn new_without_event(row: usize, col: usize) -> Self {
        Self { row, col }
    }

    #[inline]
    pub fn move_to(&mut self, row: usize, col: usize, sender: &Sender<ChangeEvent>) {
        if self.row == row && self.col == col {
            return;
        }
        let from = *self;
        self.row = row;
        self.col = col;
        let event = ChangeEvent::MoveCaret { from, to: *self };
        sender.send(event).unwrap();
    }

    pub fn to(&mut self, to: &Caret, sender: &Sender<ChangeEvent>) {
        self.move_to(to.row, to.col, sender);
    }
}
