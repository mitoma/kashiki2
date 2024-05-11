use std::sync::mpsc::Sender;

use crate::{
    buffer::{BufferChar, CellPosition},
    editor::ChangeEvent,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Caret {
    pub position: CellPosition,
    pub caret_type: CaretType,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum CaretType {
    Primary,
    Mark,
}

impl Caret {
    pub fn new(position: CellPosition, sender: &Sender<ChangeEvent>) -> Self {
        let instance = Self {
            position,
            caret_type: CaretType::Primary,
        };
        sender.send(ChangeEvent::AddCaret(instance)).unwrap();
        instance
    }

    pub fn new_mark(position: CellPosition, sender: &Sender<ChangeEvent>) -> Self {
        let instance = Self {
            position,
            caret_type: CaretType::Mark,
        };
        sender.send(ChangeEvent::AddCaret(instance)).unwrap();
        instance
    }

    pub fn new_without_event(position: CellPosition, caret_type: CaretType) -> Self {
        Self {
            position,
            caret_type,
        }
    }

    #[inline]
    pub fn move_to(&mut self, position: CellPosition, sender: &Sender<ChangeEvent>) {
        if self.position == position {
            return;
        }
        let from = *self;
        self.position = position;
        let event = ChangeEvent::MoveCaret { from, to: *self };
        sender.send(event).unwrap();
    }

    pub fn to(&mut self, to: &Caret, sender: &Sender<ChangeEvent>) {
        self.move_to(to.position, sender);
    }

    pub fn is_same_or_after_on_row(&self, c: &BufferChar) -> bool {
        self.position.is_same_or_after_on_row(&c.position)
    }
}
