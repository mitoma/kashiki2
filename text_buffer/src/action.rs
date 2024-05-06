use std::sync::mpsc::Sender;

use crate::buffer::*;
use crate::caret::*;
use crate::editor::ChangeEvent;

#[derive(Debug, PartialEq, Eq)]
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

    ForwardWord,
    BackWord,

    DeleteWord,
    BackspaceWord,

    InsertString(String),
    InsertChar(char),
    InsertEnter,
    Backspace,
    Delete,

    Undo,
    Noop,

    Copy(fn(String)),
    Cut(fn(String)),
    Mark,
    UnMark,
}

impl EditorOperation {
    // unmark が必要なオペレーションかどうかを判定する
    // バッファに対する変更処理を処理を行った後は
    // mark のポジションがズレていいことなしなので unmark する
    // 将来維持したくなったら変更処理時に mark のポジションを調整する必要がある
    #[inline]
    pub(crate) fn is_unmark_operation(&self) -> bool {
        matches!(
            self,
            EditorOperation::InsertString(_)
                | EditorOperation::InsertChar(_)
                | EditorOperation::InsertEnter
                | EditorOperation::Backspace
                | EditorOperation::BackspaceWord
                | EditorOperation::Delete
                | EditorOperation::DeleteWord
                | EditorOperation::Copy(_)
                | EditorOperation::Cut(_)
                | EditorOperation::UnMark
        )
    }
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
            ReverseAction::MoveTo(caret) => EditorOperation::MoveTo(*caret),
            ReverseAction::Back => EditorOperation::Back,
            ReverseAction::Backspace => EditorOperation::Backspace,
            ReverseAction::InsertChar(c) => EditorOperation::InsertChar(*c),
            ReverseAction::InsertString(str) => EditorOperation::InsertString(str.clone()),
            ReverseAction::InsertEnter => EditorOperation::InsertEnter,
        }
    }
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
        mark_caret: &mut Option<Caret>,
        reverse_actions: &ReverseActions,
        sender: &Sender<ChangeEvent>,
    ) -> ReverseActions {
        let mut reverse_reverse_actions = ReverseActions::default();
        for action in reverse_actions.actions.iter() {
            let reverse_reverse_action = BufferApplyer::apply_action(
                buffer,
                current_caret,
                mark_caret,
                &action.to_editor_operation(),
                sender,
            );
            reverse_reverse_action
                .actions
                .into_iter()
                .for_each(|reverse_action| reverse_reverse_actions.push(reverse_action));
        }
        reverse_reverse_actions
    }

    pub fn apply_action(
        buffer: &mut Buffer,
        current_caret: &mut Caret,
        mark_caret: &mut Option<Caret>,
        action: &EditorOperation,
        sender: &Sender<ChangeEvent>,
    ) -> ReverseActions {
        let mut reverse_actions = ReverseActions::default();
        match action {
            // move caret
            EditorOperation::MoveTo(next_caret) => {
                reverse_actions.push(ReverseAction::MoveTo(*current_caret));
                current_caret.to(next_caret, sender);
            }
            EditorOperation::Head => {
                reverse_actions.push(ReverseAction::MoveTo(*current_caret));
                buffer.head(current_caret);
            }
            EditorOperation::Last => {
                reverse_actions.push(ReverseAction::MoveTo(*current_caret));
                buffer.last(current_caret);
            }
            EditorOperation::Back => {
                reverse_actions.push(ReverseAction::MoveTo(*current_caret));
                buffer.back(current_caret);
            }
            EditorOperation::BackWord => {
                reverse_actions.push(ReverseAction::MoveTo(*current_caret));
                buffer.back_word(current_caret);
            }
            EditorOperation::Forward => {
                reverse_actions.push(ReverseAction::MoveTo(*current_caret));
                buffer.forward(current_caret);
            }
            EditorOperation::ForwardWord => {
                reverse_actions.push(ReverseAction::MoveTo(*current_caret));
                buffer.forward_word(current_caret);
            }
            EditorOperation::Previous => {
                reverse_actions.push(ReverseAction::MoveTo(*current_caret));
                buffer.previous(current_caret);
            }
            EditorOperation::Next => {
                reverse_actions.push(ReverseAction::MoveTo(*current_caret));
                buffer.next(current_caret);
            }
            EditorOperation::BufferHead => {
                reverse_actions.push(ReverseAction::MoveTo(*current_caret));
                buffer.buffer_head(current_caret);
            }
            EditorOperation::BufferLast => {
                reverse_actions.push(ReverseAction::MoveTo(*current_caret));
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
                let str_value = str_value.clone().replace("\r\n", "\n").replace('\r', "\n");
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
            EditorOperation::DeleteWord => {
                let pre_caret = *current_caret;
                buffer.forward_word(current_caret);
                if pre_caret.position != current_caret.position {
                    let text = buffer.copy_string(&pre_caret, current_caret);
                    reverse_actions
                        .actions
                        .push(ReverseAction::InsertString(text));
                    reverse_actions.push(ReverseAction::MoveTo(pre_caret));
                    loop {
                        if pre_caret.position == current_caret.position {
                            break;
                        }
                        let _removed_char = buffer.backspace(current_caret);
                    }
                }
            }
            EditorOperation::BackspaceWord => {
                let mut pre_caret = *current_caret;
                buffer.back_word(current_caret);
                if pre_caret.position != current_caret.position {
                    let text = buffer.copy_string(&pre_caret, current_caret);
                    reverse_actions
                        .actions
                        .push(ReverseAction::InsertString(text));
                    reverse_actions.push(ReverseAction::MoveTo(pre_caret));
                    loop {
                        if pre_caret.position == current_caret.position {
                            break;
                        }
                        let _removed_char = buffer.backspace(&mut pre_caret);
                    }
                }
            }
            EditorOperation::Copy(func) => {
                if let Some(mark_caret) = mark_caret {
                    let text = buffer.copy_string(mark_caret, current_caret);
                    func(text);
                }
            }
            EditorOperation::Cut(func) => {
                if let Some(mark_caret) = mark_caret {
                    let text = buffer.copy_string(mark_caret, current_caret);
                    func(text.clone());
                    reverse_actions
                        .actions
                        .push(ReverseAction::InsertString(text));
                    let (from, to) = if mark_caret < current_caret {
                        (mark_caret, current_caret)
                    } else {
                        (current_caret, mark_caret)
                    };
                    loop {
                        if from.position == to.position {
                            break;
                        }
                        let _removed_char = buffer.backspace(to);
                    }
                }
            }
            EditorOperation::Noop => {}
            EditorOperation::Undo => {}
            EditorOperation::Mark => {}
            EditorOperation::UnMark => {}
        };
        reverse_actions
    }
}

#[cfg(test)]
mod tests {

    use std::sync::mpsc::channel;

    use super::*;

    #[test]
    fn test_buffer_move() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let mut sut = Buffer::new(tx.clone());
        let mut caret = Caret::new([0, 0].into(), &tx);
        BufferApplyer::apply_action(
            &mut sut,
            &mut caret,
            &mut None,
            &EditorOperation::InsertString("ABCD\nEFGH\nIJKL\nMNO".to_string()),
            &tx,
        );
        assert_eq!(caret.position, [3, 3].into());
        let result = BufferApplyer::apply_action(
            &mut sut,
            &mut caret,
            &mut None,
            &EditorOperation::Head,
            &tx,
        );
        assert_eq!(caret.position, [3, 0].into());
        BufferApplyer::apply_reserve_actions(&mut sut, &mut caret, &mut None, &result, &tx);
        assert_eq!(caret.position, [3, 3].into());
    }

    #[test]
    fn test_apply_action() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let mut sut = Buffer::new(tx.clone());
        let mut caret = Caret::new([0, 0].into(), &tx);
        let mut reverses = Vec::new();
        let result = BufferApplyer::apply_action(
            &mut sut,
            &mut caret,
            &mut None,
            &EditorOperation::InsertChar('花'),
            &tx,
        );
        reverses.push(result);
        let result = BufferApplyer::apply_action(
            &mut sut,
            &mut caret,
            &mut None,
            &EditorOperation::InsertChar('鳥'),
            &tx,
        );
        reverses.push(result);
        let result = BufferApplyer::apply_action(
            &mut sut,
            &mut caret,
            &mut None,
            &EditorOperation::InsertEnter,
            &tx,
        );
        reverses.push(result);
        assert_eq!(sut.to_buffer_string(), "花鳥\n".to_string());
        assert_eq!(caret, Caret::new([1, 0].into(), &tx));

        let reverse_action = reverses.pop().unwrap();
        BufferApplyer::apply_reserve_actions(&mut sut, &mut caret, &mut None, &reverse_action, &tx);
        assert_eq!(sut.to_buffer_string(), "花鳥".to_string());

        let reverse_action = reverses.pop().unwrap();
        BufferApplyer::apply_reserve_actions(&mut sut, &mut caret, &mut None, &reverse_action, &tx);
        assert_eq!(sut.to_buffer_string(), "花".to_string());

        let reverse_action = reverses.pop().unwrap();
        BufferApplyer::apply_reserve_actions(&mut sut, &mut caret, &mut None, &reverse_action, &tx);
        assert_eq!(sut.to_buffer_string(), "".to_string());
        assert_eq!(caret, Caret::new([0, 0].into(), &tx));
    }

    #[test]
    fn test_copy() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let mut sut = Buffer::new(tx.clone());
        BufferApplyer::apply_action(
            &mut sut,
            &mut Caret::new([0, 0].into(), &tx),
            &mut None,
            &EditorOperation::InsertString("ABCD\nEFGH\nIJKL\nMNO".to_string()),
            &tx,
        );

        BufferApplyer::apply_action(
            &mut sut,
            &mut Caret::new([1, 1].into(), &tx),
            &mut Some(Caret::new([2, 2].into(), &tx)),
            &EditorOperation::Copy(|text| {
                assert_eq!(text, "FGH\nIJ".to_string());
            }),
            &tx,
        );
    }
}
