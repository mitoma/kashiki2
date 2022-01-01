use super::action::*;
use super::buffer::*;
use super::caret::*;

pub struct Editor {
    main_caret: Caret,
    mark: Option<Caret>,
    buffer: Buffer,
    undo_list: Vec<ReverseActions>,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            main_caret: Caret::new(0, 0),
            mark: Option::None,
            buffer: Buffer::default(),
            undo_list: Vec::new(),
        }
    }
}

impl Editor {
    pub fn operation(&mut self, op: &EditorOperation) {
        if let EditorOperation::Undo = op {
            self.undo();
            return;
        }
        let reverse_actions =
            BufferApplyer::apply_action(&mut self.buffer, &mut self.main_caret, op);
        self.undo_list.push(reverse_actions);
    }

    fn undo(&mut self) {
        if let Some(reverse_action) = self.undo_list.pop() {
            BufferApplyer::apply_reserve_actions(
                &mut self.buffer,
                &mut self.main_caret,
                &reverse_action,
            );
        }
    }

    pub fn mark(&mut self) {
        self.mark = Some(self.main_caret.clone());
    }

    pub fn to_buffer_string(&self) -> String {
        self.buffer.to_buffer_string()
    }
}
