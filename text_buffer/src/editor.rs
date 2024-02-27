use std::sync::mpsc::Sender;

use super::action::*;
use super::buffer::*;
use super::caret::*;

pub struct Editor {
    main_caret: Caret,
    mark: Option<Caret>,
    buffer: Buffer,
    undo_list: Vec<ReverseActions>,
    sender: Sender<ChangeEvent>,
}

impl Editor {
    pub fn new(sender: Sender<ChangeEvent>) -> Self {
        Self {
            main_caret: Caret::new(0, 0, &sender),
            mark: Option::None,
            buffer: Buffer::new(sender.clone()),
            undo_list: Vec::new(),
            sender,
        }
    }

    pub fn operation(&mut self, op: &EditorOperation) {
        if EditorOperation::Undo == *op {
            self.undo();
            return;
        }
        if EditorOperation::Mark == *op {
            self.mark();
            return;
        }
        let reverse_actions = BufferApplyer::apply_action(
            &mut self.buffer,
            &mut self.main_caret,
            self.mark.as_mut(),
            op,
            &self.sender,
        );
        self.undo_list.push(reverse_actions);
    }

    fn undo(&mut self) {
        if let Some(reverse_action) = self.undo_list.pop() {
            BufferApplyer::apply_reserve_actions(
                &mut self.buffer,
                &mut self.main_caret,
                self.mark.as_mut(),
                &reverse_action,
                &self.sender,
            );
        }
    }

    pub fn mark(&mut self) {
        if let Some(current_mark) = self.mark {
            self.sender
                .send(ChangeEvent::RemoveCaret(current_mark))
                .unwrap();
        }
        self.mark = Some(Caret::new(
            self.main_caret.row,
            self.main_caret.col,
            &self.sender,
        ));
    }

    pub fn to_buffer_string(&self) -> String {
        self.buffer.to_buffer_string()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ChangeEvent {
    AddChar(BufferChar),
    MoveChar { from: BufferChar, to: BufferChar },
    RemoveChar(BufferChar),
    AddCaret(Caret),
    MoveCaret { from: Caret, to: Caret },
    RemoveCaret(Caret),
}
