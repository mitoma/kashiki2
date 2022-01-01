use crate::buffer::*;
use crate::caret::*;

#[derive(Debug)]
pub enum EditorOperation {
    MoveTo(Caret),

    Head,
    Last,
    Back,
    Forward,
    Previous,
    Next,
    BufferHead,
    BufferLast,

    InsertString(String),
    InsertChar(char),
    InsertEnter,
    Backspace,
    Delete,

    Undo,
    Noop,
}

#[derive(Debug)]
pub enum ReverseAction {
    MoveTo(Caret),

    Back,
    InsertString(String),
    InsertChar(char),
    InsertEnter,
    Backspace,
}

impl ReverseAction {
    fn to_editor_operation(&self) -> EditorOperation {
        match self {
            ReverseAction::MoveTo(caret) => EditorOperation::MoveTo(caret.clone()),
            ReverseAction::Back => EditorOperation::Back,
            ReverseAction::Backspace => EditorOperation::Backspace,
            ReverseAction::InsertChar(c) => EditorOperation::InsertChar(*c),
            ReverseAction::InsertString(str) => EditorOperation::InsertString(str.clone()),
            ReverseAction::InsertEnter => EditorOperation::InsertEnter,
        }
    }
}

pub enum BufferStateAction {
    Mark(),
    Copy(),
}

#[derive(Debug, Default)]
pub struct ReverseActions {
    actions: Vec<ReverseAction>,
}

impl ReverseActions {
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub fn push(&mut self, action: ReverseAction) {
        self.actions.push(action);
    }
}

pub struct BufferApplyer {}

impl BufferApplyer {
    pub fn apply_reserve_actions(
        buffer: &mut Buffer,
        current_caret: &mut Caret,
        reverse_actions: &ReverseActions,
    ) -> ReverseActions {
        let mut reverse_reverse_actions = ReverseActions::default();
        reverse_actions.actions.iter().for_each(|action| {
            let reverse_reverse_action =
                BufferApplyer::apply_action(buffer, current_caret, &action.to_editor_operation());
            reverse_reverse_action
                .actions
                .into_iter()
                .for_each(|reverse_action| reverse_reverse_actions.push(reverse_action));
        });
        reverse_reverse_actions
    }

    pub fn apply_action(
        buffer: &mut Buffer,
        current_caret: &mut Caret,
        action: &EditorOperation,
    ) -> ReverseActions {
        let mut reverse_actions = ReverseActions::default();
        match action {
            // move caret
            EditorOperation::MoveTo(next_caret) => {
                reverse_actions.push(ReverseAction::MoveTo(current_caret.clone()));
                current_caret.to(next_caret);
            }
            EditorOperation::Head => {
                reverse_actions.push(ReverseAction::MoveTo(current_caret.clone()));
                buffer.head(current_caret);
            }
            EditorOperation::Last => {
                reverse_actions.push(ReverseAction::MoveTo(current_caret.clone()));
                buffer.last(current_caret);
            }
            EditorOperation::Back => {
                reverse_actions.push(ReverseAction::MoveTo(current_caret.clone()));
                buffer.back(current_caret);
            }
            EditorOperation::Forward => {
                reverse_actions.push(ReverseAction::MoveTo(current_caret.clone()));
                buffer.forward(current_caret);
            }
            EditorOperation::Previous => {
                reverse_actions.push(ReverseAction::MoveTo(current_caret.clone()));
                buffer.previous(current_caret);
            }
            EditorOperation::Next => {
                reverse_actions.push(ReverseAction::MoveTo(current_caret.clone()));
                buffer.next(current_caret);
            }
            EditorOperation::BufferHead => {
                reverse_actions.push(ReverseAction::MoveTo(current_caret.clone()));
                buffer.buffer_head(current_caret);
            }
            EditorOperation::BufferLast => {
                reverse_actions.push(ReverseAction::MoveTo(current_caret.clone()));
                buffer.buffer_last(current_caret);
            }

            // modify buffer
            EditorOperation::InsertEnter => {
                buffer.insert_enter(current_caret);
                reverse_actions.push(ReverseAction::Backspace);
            }
            EditorOperation::InsertChar(char_value) => {
                buffer.insert_char(current_caret, *char_value);
                reverse_actions.push(ReverseAction::Backspace);
            }
            EditorOperation::InsertString(str_value) => {
                // normalize
                let str_value = str_value.clone().replace("\r\n", "\n").replace("\r", "\n");
                buffer.insert_string(current_caret, str_value.clone());
                str_value.chars().for_each(|_| {
                    reverse_actions.push(ReverseAction::Backspace);
                });
            }
            EditorOperation::Backspace => {
                let removed_char = buffer.backspace(current_caret);
                match removed_char {
                    RemovedChar::Char(c) => {
                        reverse_actions.actions.push(ReverseAction::InsertChar(c))
                    }
                    RemovedChar::Enter => reverse_actions.actions.push(ReverseAction::InsertEnter),
                    RemovedChar::None => {}
                }
            }
            EditorOperation::Delete => {
                let removed_char = buffer.delete(current_caret);
                match removed_char {
                    RemovedChar::Char(c) => {
                        reverse_actions.actions.push(ReverseAction::InsertChar(c));
                        reverse_actions.actions.push(ReverseAction::Back);
                    }
                    RemovedChar::Enter => {
                        reverse_actions.actions.push(ReverseAction::InsertEnter);
                        reverse_actions.actions.push(ReverseAction::Back);
                    }
                    RemovedChar::None => {}
                }
            }
            EditorOperation::Noop => {}
            EditorOperation::Undo => {}
        };
        reverse_actions
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_buffer_move() {
        let mut sut = Buffer::default();
        let mut caret = Caret::new(0, 0);
        BufferApplyer::apply_action(
            &mut sut,
            &mut caret,
            &EditorOperation::InsertString("ABCD\nEFGH\nIJKL\nMNO".to_string()),
        );
        assert_eq!(caret, Caret::new(3, 3));
        let result = BufferApplyer::apply_action(&mut sut, &mut caret, &EditorOperation::Head);
        assert_eq!(caret, Caret::new(3, 0));
        BufferApplyer::apply_reserve_actions(&mut sut, &mut caret, &result);
        assert_eq!(caret, Caret::new(3, 3));
    }

    #[test]
    fn test_apply_action() {
        let mut sut = Buffer::default();
        let mut caret = Caret::new(0, 0);
        let mut reverses = Vec::new();
        let result =
            BufferApplyer::apply_action(&mut sut, &mut caret, &EditorOperation::InsertChar('花'));
        reverses.push(result);
        let result =
            BufferApplyer::apply_action(&mut sut, &mut caret, &EditorOperation::InsertChar('鳥'));
        reverses.push(result);
        let result =
            BufferApplyer::apply_action(&mut sut, &mut caret, &EditorOperation::InsertEnter);
        reverses.push(result);
        assert_eq!(sut.to_buffer_string(), "花鳥\n".to_string());
        assert_eq!(caret, Caret::new(1, 0));

        let reverse_action = reverses.pop().unwrap();
        BufferApplyer::apply_reserve_actions(&mut sut, &mut caret, &reverse_action);
        assert_eq!(sut.to_buffer_string(), "花鳥".to_string());

        let reverse_action = reverses.pop().unwrap();
        BufferApplyer::apply_reserve_actions(&mut sut, &mut caret, &reverse_action);
        assert_eq!(sut.to_buffer_string(), "花".to_string());

        let reverse_action = reverses.pop().unwrap();
        BufferApplyer::apply_reserve_actions(&mut sut, &mut caret, &reverse_action);
        assert_eq!(sut.to_buffer_string(), "".to_string());
        assert_eq!(caret, Caret::new(0, 0));
    }
}
