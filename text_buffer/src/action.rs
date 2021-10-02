use crate::buffer::*;
use crate::caret::*;

#[derive(Debug)]
pub enum BufferAction {
    MoveTo(Caret),

    Head(Caret),
    Last(Caret),
    Back(Caret),
    Forward(Caret),
    Previous(Caret),
    Next(Caret),
    BufferHead(Caret),
    BufferLast(Caret),

    InsertString(Caret, String),
    InsertChar(Caret, char),
    InsertEnter(Caret),
    Backspace(Caret),
    Delete(Caret),

    Noop(Caret),
}

pub enum BufferStateAction {
    Mark(),
    Copy(),
}

#[derive(Debug)]
pub struct ReverseAction {
    actions: Vec<BufferAction>,
}

impl Default for ReverseAction {
    fn default() -> Self {
        Self {
            actions: Vec::new(),
        }
    }
}
impl ReverseAction {
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

#[derive(Debug)]
pub struct ApplyResult {
    pub caret: Caret,
    pub reverse_action: ReverseAction,
}

impl ApplyResult {
    fn new(caret: Caret, reverse_action: ReverseAction) -> Self {
        Self {
            caret,
            reverse_action,
        }
    }
}

pub struct BufferApplyer {}

impl BufferApplyer {
    pub fn apply_reserve_actions(
        buffer: &mut Buffer,
        reverse_action: &ReverseAction,
    ) -> ApplyResult {
        reverse_action.actions.iter().fold(
            ApplyResult::new(Caret::new(0, 0), ReverseAction::default()),
            |mut result, action| {
                let mut r = BufferApplyer::apply_action(buffer, &action);
                result.caret = r.caret;
                result
                    .reverse_action
                    .actions
                    .append(&mut r.reverse_action.actions);
                result
            },
        )
    }

    pub fn apply_action(buffer: &mut Buffer, action: &BufferAction) -> ApplyResult {
        let mut result = ReverseAction {
            actions: Vec::new(),
        };

        match action {
            BufferAction::MoveTo(caret) => {
                // MoveTo には from と to 二つ必要かどうかよくわかっていない。
                result.actions.push(BufferAction::MoveTo(caret.clone()));
                ApplyResult::new(caret.clone(), result)
            }
            BufferAction::Head(caret) => {
                let caret = &mut caret.clone();
                result.actions.push(BufferAction::MoveTo(caret.clone()));
                buffer.head(caret);
                ApplyResult::new(caret.clone(), result)
            }
            BufferAction::Last(caret) => {
                let caret = &mut caret.clone();
                result.actions.push(BufferAction::MoveTo(caret.clone()));
                buffer.last(caret);
                ApplyResult::new(caret.clone(), result)
            }
            BufferAction::Back(caret) => {
                let caret = &mut caret.clone();
                result.actions.push(BufferAction::MoveTo(caret.clone()));
                buffer.back(caret);
                ApplyResult::new(caret.clone(), result)
            }
            BufferAction::Forward(caret) => {
                let caret = &mut caret.clone();
                result.actions.push(BufferAction::MoveTo(caret.clone()));
                buffer.forward(caret);
                ApplyResult::new(caret.clone(), result)
            }
            BufferAction::Previous(caret) => {
                let caret = &mut caret.clone();
                result.actions.push(BufferAction::MoveTo(caret.clone()));
                buffer.previous(caret);
                ApplyResult::new(caret.clone(), result)
            }
            BufferAction::Next(caret) => {
                let caret = &mut caret.clone();
                result.actions.push(BufferAction::MoveTo(caret.clone()));
                buffer.next(caret);
                ApplyResult::new(caret.clone(), result)
            }
            BufferAction::BufferHead(caret) => {
                let caret = &mut caret.clone();
                result.actions.push(BufferAction::MoveTo(caret.clone()));
                buffer.buffer_head(caret);
                ApplyResult::new(caret.clone(), result)
            }
            BufferAction::BufferLast(caret) => {
                let caret = &mut caret.clone();
                result.actions.push(BufferAction::MoveTo(caret.clone()));
                buffer.buffer_last(caret);
                ApplyResult::new(caret.clone(), result)
            }

            BufferAction::InsertEnter(caret) => {
                let caret = &mut caret.clone();
                buffer.insert_enter(caret);
                result.actions.push(BufferAction::Backspace(caret.clone()));
                ApplyResult::new(caret.clone(), result)
            }
            BufferAction::InsertChar(caret, char_value) => {
                let caret = &mut caret.clone();
                buffer.insert_char(caret, char_value.clone());
                result.actions.push(BufferAction::Backspace(caret.clone()));
                ApplyResult::new(caret.clone(), result)
            }
            BufferAction::InsertString(caret, str_value) => {
                let pre_caret = caret.clone();
                let caret = &mut caret.clone();
                result.actions.push(BufferAction::MoveTo(caret.clone()));
                buffer.insert_string(caret, str_value.clone());
                for _ in 0..str_value.chars().count() {
                    result.actions.push(BufferAction::Delete(pre_caret.clone()));
                }
                ApplyResult::new(caret.clone(), result)
            }
            BufferAction::Backspace(caret) => {
                let caret = &mut caret.clone();
                let removed_char = buffer.backspace(caret);
                match removed_char {
                    RemovedChar::Char(c) => result
                        .actions
                        .push(BufferAction::InsertChar(caret.clone(), c)),
                    RemovedChar::Enter => result
                        .actions
                        .push(BufferAction::InsertEnter(caret.clone())),
                    RemovedChar::None => {}
                }
                ApplyResult::new(caret.clone(), result)
            }
            BufferAction::Delete(pre_caret) => {
                let removed_char = buffer.delete(pre_caret);
                match removed_char {
                    RemovedChar::Char(c) => {
                        result
                            .actions
                            .push(BufferAction::InsertChar(pre_caret.clone(), c));
                        result.actions.push(BufferAction::MoveTo(pre_caret.clone()));
                    }
                    RemovedChar::Enter => {
                        result
                            .actions
                            .push(BufferAction::InsertEnter(pre_caret.clone()));
                        result.actions.push(BufferAction::MoveTo(pre_caret.clone()));
                    }
                    RemovedChar::None => {}
                }
                ApplyResult::new(pre_caret.clone(), result)
            }
            BufferAction::Noop(pre_caret) => ApplyResult::new(pre_caret.clone(), result),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_buffer_move() {
        let mut sut = Buffer::new();
        sut.insert_string(&mut Caret::new(0, 0), "ABCD\nEFGH\nIJKL\nMNO".to_string());
        let result = BufferApplyer::apply_action(&mut sut, &BufferAction::Last(Caret::new(0, 0)));
        assert_eq!(result.caret, Caret::new(0, 4));
        let result = BufferApplyer::apply_reserve_actions(&mut sut, &result.reverse_action);
        assert_eq!(result.caret, Caret::new(0, 0));
    }

    #[test]
    fn test_apply_action() {
        let mut sut = Buffer::new();
        let mut reverses = Vec::new();
        let result = BufferApplyer::apply_action(
            &mut sut,
            &BufferAction::InsertChar(Caret::new(0, 0), '花'),
        );
        reverses.push(result.reverse_action);
        let result =
            BufferApplyer::apply_action(&mut sut, &BufferAction::InsertChar(result.caret, '鳥'));
        reverses.push(result.reverse_action);
        let result =
            BufferApplyer::apply_action(&mut sut, &BufferAction::InsertChar(result.caret, '\n'));
        reverses.push(result.reverse_action);
        assert_eq!(sut.to_buffer_string(), "花鳥\n".to_string());

        let reverse_action = reverses.pop().unwrap();
        reverse_action.actions.iter().for_each(|action| {
            BufferApplyer::apply_action(&mut sut, action);
        });
        assert_eq!(sut.to_buffer_string(), "花鳥".to_string());

        let reverse_action = reverses.pop().unwrap();
        reverse_action.actions.iter().for_each(|action| {
            BufferApplyer::apply_action(&mut sut, action);
        });
        assert_eq!(sut.to_buffer_string(), "花".to_string());

        let reverse_action = reverses.pop().unwrap();
        reverse_action.actions.iter().for_each(|action| {
            BufferApplyer::apply_action(&mut sut, action);
        });
        assert_eq!(sut.to_buffer_string(), "".to_string());
    }
}
