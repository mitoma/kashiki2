use super::buffer::*;

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

pub fn buffer_apply(mut buffer: Buffer, action: &BufferAction) -> (Buffer, Caret, ReverseAction) {
    let mut result = ReverseAction {
        actions: Vec::new(),
    };

    match action {
        BufferAction::MoveTo(caret) => {
            // MoveTo には from と to 二つ必要かどうかよくわかっていない。
            result.actions.push(BufferAction::MoveTo(caret.clone()));
            (buffer, caret.clone(), result)
        }
        BufferAction::Head(caret) => {
            result.actions.push(BufferAction::MoveTo(caret.clone()));
            let caret = buffer.head(caret.clone());
            (buffer, caret, result)
        }
        BufferAction::Last(caret) => {
            result.actions.push(BufferAction::MoveTo(caret.clone()));
            let caret = buffer.last(caret.clone());
            (buffer, caret, result)
        }
        BufferAction::Back(caret) => {
            result.actions.push(BufferAction::MoveTo(caret.clone()));
            let caret = buffer.back(caret.clone());
            (buffer, caret, result)
        }
        BufferAction::Forward(caret) => {
            result.actions.push(BufferAction::MoveTo(caret.clone()));
            let caret = buffer.forward(caret.clone());
            (buffer, caret, result)
        }
        BufferAction::Previous(caret) => {
            result.actions.push(BufferAction::MoveTo(caret.clone()));
            let caret = buffer.previous(caret.clone());
            (buffer, caret, result)
        }
        BufferAction::Next(caret) => {
            result.actions.push(BufferAction::MoveTo(caret.clone()));
            let caret = buffer.next(caret.clone());
            (buffer, caret, result)
        }
        BufferAction::BufferHead(caret) => {
            result.actions.push(BufferAction::MoveTo(caret.clone()));
            let caret = buffer.buffer_head(caret.clone());
            (buffer, caret, result)
        }
        BufferAction::BufferLast(caret) => {
            result.actions.push(BufferAction::MoveTo(caret.clone()));
            let caret = buffer.buffer_last(caret.clone());
            (buffer, caret, result)
        }

        BufferAction::InsertEnter(caret) => {
            let caret = buffer.insert_enter(caret.clone());
            result.actions.push(BufferAction::Backspace(caret.clone()));
            (buffer, caret, result)
        }
        BufferAction::InsertChar(caret, c) => {
            let caret = buffer.insert_char(caret.clone(), c.clone());
            result.actions.push(BufferAction::Backspace(caret.clone()));
            (buffer, caret, result)
        }
        BufferAction::Backspace(caret) => {
            let caret = buffer.backspace(caret.clone());
            // ここも書く必要ある
            (buffer, caret, result)
        }

        _ => {
            /* メチャ適当。ここを通ればバグる */
            (buffer, Caret::new(0, 0), result)
        }
    }
}
#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_buffer_apply() {
        let sut = Buffer::new("hello buffer".to_string());
        let mut reverses = Vec::new();
        let (sut, caret, action) =
            buffer_apply(sut, &BufferAction::InsertChar(Caret::new(0, 0), '花'));
        reverses.push(action);
        let (sut, caret, action) = buffer_apply(sut, &BufferAction::InsertChar(caret, '鳥'));
        reverses.push(action);
        let (sut, caret, action) = buffer_apply(sut, &BufferAction::InsertChar(caret, '\n'));
        reverses.push(action);
        assert_eq!(sut.to_buffer_string(), "花鳥\n".to_string());

        let reverse_action = reverses.pop().unwrap();
        let sut = reverse_action.actions.iter().fold(sut, |sut, action| {
            let (sut, _, _) = buffer_apply(sut, action);
            sut
        });
        assert_eq!(sut.to_buffer_string(), "花鳥".to_string());

        let reverse_action = reverses.pop().unwrap();
        let sut = reverse_action.actions.iter().fold(sut, |sut, action| {
            let (sut, _, _) = buffer_apply(sut, action);
            sut
        });
        assert_eq!(sut.to_buffer_string(), "花".to_string());

        let reverse_action = reverses.pop().unwrap();
        let sut = reverse_action.actions.iter().fold(sut, |sut, action| {
            let (sut, _, _) = buffer_apply(sut, action);
            sut
        });
        assert_eq!(sut.to_buffer_string(), "".to_string());
    }
}
