use super::action::*;
use super::buffer::*;
use super::caret::*;

pub struct BufferState {
    main_caret: Caret,
    mark: Option<Caret>,
    buffer: Buffer,
    reverse_actions: Vec<ReverseAction>,
}

impl BufferState {
    fn new(buffer_name: String) -> BufferState {
        BufferState {
            main_caret: Caret::new(0, 0),
            mark: Option::None,
            buffer: Buffer::new(buffer_name),
            reverse_actions: Vec::new(),
        }
    }
}
