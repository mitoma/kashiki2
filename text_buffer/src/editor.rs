use super::action::*;
use super::buffer::*;
use super::caret::*;

pub struct Editor {
    main_caret: Caret,
    mark: Option<Caret>,
    buffer: Buffer,
    reverse_actions: Vec<ReverseActions>,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            main_caret: Caret::new(0, 0),
            mark: Option::None,
            buffer: Buffer::new(),
            reverse_actions: Vec::new(),
        }
    }
}

impl Editor {
    pub fn operation(&mut self, op: &EditorOperation) {
        BufferApplyer::apply_action(&mut self.buffer, &mut self.main_caret, op);
    }

    pub fn mark(&mut self) {
        self.mark = Some(self.main_caret.clone());
    }

    pub fn to_buffer_string(&self) -> String {
        self.buffer.to_buffer_string()
    }
}
