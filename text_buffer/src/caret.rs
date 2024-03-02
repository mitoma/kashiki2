use std::sync::mpsc::Sender;

use crate::{buffer::BufferChar, editor::ChangeEvent};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Caret {
    pub row: usize,
    pub col: usize,
    pub caret_type: CaretType,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum CaretType {
    Primary,
    Mark,
}

impl Caret {
    pub fn new(row: usize, col: usize, sender: &Sender<ChangeEvent>) -> Self {
        let instance = Self {
            row,
            col,
            caret_type: CaretType::Primary,
        };
        sender.send(ChangeEvent::AddCaret(instance)).unwrap();
        instance
    }

    pub fn new_mark(row: usize, col: usize, sender: &Sender<ChangeEvent>) -> Self {
        let instance = Self {
            row,
            col,
            caret_type: CaretType::Mark,
        };
        sender.send(ChangeEvent::AddCaret(instance)).unwrap();
        instance
    }

    pub fn new_without_event(row: usize, col: usize, caret_type: CaretType) -> Self {
        Self {
            row,
            col,
            caret_type,
        }
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

    pub fn is_same_or_after_on_row(&self, c: &BufferChar) -> bool {
        self.row == c.row && self.col >= c.col
    }
}
