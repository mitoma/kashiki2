use crate::caret::*;
use crate::buffer::*;

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
}

pub struct ReverseAction {
    actions: Vec<BufferAction>,
}

impl ReverseAction {
    fn new() -> ReverseAction {
        ReverseAction {
            actions: Vec::new(),
        }
    }
}

pub struct ApplyResult {
    buffer: Buffer,
    caret: Caret,
    reverse_action: ReverseAction,
}

impl ApplyResult {
    fn new(buffer: Buffer, caret: Caret, reverse_action: ReverseAction) -> ApplyResult {
        ApplyResult {
            buffer: buffer,
            caret: caret,
            reverse_action: reverse_action,
        }
    }
}

pub fn apply_reserve_actions(buffer: Buffer, reverse_action: &ReverseAction) -> ApplyResult {
    reverse_action.actions.iter().fold(
        ApplyResult::new(buffer, Caret::new(0, 0), ReverseAction::new()),
        |mut result, action| {
            let mut r = apply_action(result.buffer, &action);
            result.buffer = r.buffer;
            result.caret = r.caret;
            result
                .reverse_action
                .actions
                .append(&mut r.reverse_action.actions);
            result
        },
    )
}

pub fn apply_action(mut buffer: Buffer, action: &BufferAction) -> ApplyResult {
    let mut result = ReverseAction {
        actions: Vec::new(),
    };

    match action {
        BufferAction::MoveTo(pre_caret) => {
            // MoveTo には from と to 二つ必要かどうかよくわかっていない。
            result.actions.push(BufferAction::MoveTo(pre_caret.clone()));
            ApplyResult::new(buffer, pre_caret.clone(), result)
        }
        BufferAction::Head(pre_caret) => {
            result.actions.push(BufferAction::MoveTo(pre_caret.clone()));
            let post_caret = buffer.head(pre_caret.clone());
            ApplyResult::new(buffer, post_caret, result)
        }
        BufferAction::Last(pre_caret) => {
            result.actions.push(BufferAction::MoveTo(pre_caret.clone()));
            let post_caret = buffer.last(pre_caret.clone());
            ApplyResult::new(buffer, post_caret, result)
        }
        BufferAction::Back(pre_caret) => {
            result.actions.push(BufferAction::MoveTo(pre_caret.clone()));
            let post_caret = buffer.back(pre_caret.clone());
            ApplyResult::new(buffer, post_caret, result)
        }
        BufferAction::Forward(pre_caret) => {
            result.actions.push(BufferAction::MoveTo(pre_caret.clone()));
            let post_caret = buffer.forward(pre_caret.clone());
            ApplyResult::new(buffer, post_caret, result)
        }
        BufferAction::Previous(pre_caret) => {
            result.actions.push(BufferAction::MoveTo(pre_caret.clone()));
            let post_caret = buffer.previous(pre_caret.clone());
            ApplyResult::new(buffer, post_caret, result)
        }
        BufferAction::Next(pre_caret) => {
            result.actions.push(BufferAction::MoveTo(pre_caret.clone()));
            let post_caret = buffer.next(pre_caret.clone());
            ApplyResult::new(buffer, post_caret, result)
        }
        BufferAction::BufferHead(pre_caret) => {
            result.actions.push(BufferAction::MoveTo(pre_caret.clone()));
            let post_caret = buffer.buffer_head(pre_caret.clone());
            ApplyResult::new(buffer, post_caret, result)
        }
        BufferAction::BufferLast(pre_caret) => {
            result.actions.push(BufferAction::MoveTo(pre_caret.clone()));
            let post_caret = buffer.buffer_last(pre_caret.clone());
            ApplyResult::new(buffer, post_caret, result)
        }

        BufferAction::InsertEnter(pre_caret) => {
            let post_caret = buffer.insert_enter(pre_caret.clone());
            result
                .actions
                .push(BufferAction::Backspace(post_caret.clone()));
            ApplyResult::new(buffer, post_caret, result)
        }
        BufferAction::InsertChar(pre_caret, char_value) => {
            let post_caret = buffer.insert_char(pre_caret.clone(), char_value.clone());
            result
                .actions
                .push(BufferAction::Backspace(post_caret.clone()));
            ApplyResult::new(buffer, post_caret, result)
        }
        BufferAction::InsertString(pre_caret, str_value) => {
            result.actions.push(BufferAction::MoveTo(pre_caret.clone()));
            let post_caret = buffer.insert_string(pre_caret.clone(), str_value.clone());
            for _ in 0..str_value.chars().count() {
                result.actions.push(BufferAction::Delete(pre_caret.clone()));
            }
            ApplyResult::new(buffer, post_caret, result)
        }
        BufferAction::Backspace(pre_caret) => {
            let (post_caret, removed_char) = buffer.backspace(pre_caret.clone());
            match removed_char {
                RemovedChar::Char(c) => result
                    .actions
                    .push(BufferAction::InsertChar(post_caret.clone(), c)),
                RemovedChar::Enter => result
                    .actions
                    .push(BufferAction::InsertEnter(post_caret.clone())),
                RemovedChar::None => {}
            }
            ApplyResult::new(buffer, post_caret, result)
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
            ApplyResult::new(buffer, pre_caret.clone(), result)
        }
    }
}
#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_buffer_move() {
        let mut sut = Buffer::new();
        sut.insert_string(Caret::new(0, 0), "ABCD\nEFGH\nIJKL\nMNO".to_string());
        let result = apply_action(sut, &BufferAction::Last(Caret::new(0, 0)));
        assert_eq!(result.caret, Caret::new(0, 4));
        let result = apply_reserve_actions(result.buffer, &result.reverse_action);
        assert_eq!(result.caret, Caret::new(0, 0));
    }

    #[test]
    fn test_apply_action() {
        let sut = Buffer::new();
        let mut reverses = Vec::new();
        let result = apply_action(sut, &BufferAction::InsertChar(Caret::new(0, 0), '花'));
        reverses.push(result.reverse_action);
        let result = apply_action(
            result.buffer,
            &BufferAction::InsertChar(result.caret, '鳥'),
        );
        reverses.push(result.reverse_action);
        let result = apply_action(result.buffer, &BufferAction::InsertChar(result.caret, '\n'));
        reverses.push(result.reverse_action);
        assert_eq!(result.buffer.to_buffer_string(), "花鳥\n".to_string());

        let reverse_action = reverses.pop().unwrap();
        let sut = reverse_action
            .actions
            .iter()
            .fold(result.buffer, |buffer, action| {
                let result = apply_action(buffer, action);
                result.buffer
            });
        assert_eq!(sut.to_buffer_string(), "花鳥".to_string());

        let reverse_action = reverses.pop().unwrap();
        let sut = reverse_action.actions.iter().fold(sut, |buffer, action| {
            let result = apply_action(buffer, action);
            result.buffer
        });
        assert_eq!(sut.to_buffer_string(), "花".to_string());

        let reverse_action = reverses.pop().unwrap();
        let sut = reverse_action.actions.iter().fold(sut, |buffer, action| {
            let result = apply_action(buffer, action);
            result.buffer
        });
        assert_eq!(sut.to_buffer_string(), "".to_string());
    }
}
