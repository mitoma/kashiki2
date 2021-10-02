use super::action::*;
use super::buffer::*;
use super::caret::*;

pub struct Editor {
    main_caret: Caret,
    mark: Option<Caret>,
    buffer: Buffer,
    reverse_actions: Vec<ReverseActions>,
}

impl Editor {
    pub fn new() -> Editor {
        Editor {
            main_caret: Caret::new(0, 0),
            mark: Option::None,
            buffer: Buffer::new(),
            reverse_actions: Vec::new(),
        }
    }

    pub fn mark(&mut self) {
        self.mark = Some(self.main_caret.clone());
    }
}
