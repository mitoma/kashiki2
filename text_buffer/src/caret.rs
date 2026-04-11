use std::{fmt::Display, sync::mpsc::Sender};

use crate::{
    buffer::{BufferChar, CellPosition},
    editor::ChangeEvent,
    notifier::{ChangeEventNotifier, notify_sender},
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

impl Display for CaretType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaretType::Primary => write!(f, "Caret"),
            CaretType::Mark => write!(f, "Mark"),
        }
    }
}

impl Caret {
    pub fn new(position: CellPosition, sender: &Sender<ChangeEvent>) -> Self {
        Self::new_with_notifier(position, CaretType::Primary, &SenderEventNotifier(sender))
    }

    pub fn new_mark(position: CellPosition, sender: &Sender<ChangeEvent>) -> Self {
        Self::new_with_notifier(position, CaretType::Mark, &SenderEventNotifier(sender))
    }

    pub(crate) fn new_primary_with_notifier(
        position: CellPosition,
        notifier: &dyn ChangeEventNotifier,
    ) -> Self {
        Self::new_with_notifier(position, CaretType::Primary, notifier)
    }

    pub(crate) fn new_mark_with_notifier(
        position: CellPosition,
        notifier: &dyn ChangeEventNotifier,
    ) -> Self {
        Self::new_with_notifier(position, CaretType::Mark, notifier)
    }

    fn new_with_notifier(
        position: CellPosition,
        caret_type: CaretType,
        notifier: &dyn ChangeEventNotifier,
    ) -> Self {
        let instance = Self {
            position,
            caret_type,
        };
        notifier.notify(ChangeEvent::AddCaret(instance));
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
        self.move_to_with_notifier(position, &SenderEventNotifier(sender));
    }

    #[inline]
    pub(crate) fn move_to_with_notifier(
        &mut self,
        position: CellPosition,
        notifier: &dyn ChangeEventNotifier,
    ) {
        if self.position == position {
            return;
        }
        let from = *self;
        self.position = position;
        let event = ChangeEvent::MoveCaret { from, to: *self };
        notifier.notify(event);
    }

    pub fn to(&mut self, to: &Caret, sender: &Sender<ChangeEvent>) {
        let _ = sender;
        self.move_to_with_notifier(to.position, &SenderEventNotifier(sender));
    }

    pub fn is_same_or_after_on_row(&self, c: &BufferChar) -> bool {
        self.position.is_same_or_after_on_row(&c.position)
    }
}

struct SenderEventNotifier<'a>(&'a Sender<ChangeEvent>);

impl ChangeEventNotifier for SenderEventNotifier<'_> {
    fn notify(&self, event: ChangeEvent) {
        notify_sender(self.0, event);
    }
}
