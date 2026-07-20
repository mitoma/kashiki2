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
            main_caret: Caret::new([0, 0].into(), &sender),
            mark: Option::None,
            buffer: Buffer::new(sender.clone()),
            undo_list: Vec::new(),
            sender,
        }
    }

    // action を実行する前後で selection が変わった場合に、変更を sender に通知する
    #[inline]
    fn action_width_selection_update(
        &mut self,
        op: &EditorOperation,
        action: impl FnOnce(&mut Self),
    ) {
        let pre_selection = self.selection();

        // unmark 対象の操作の場合は action 実行前に選択範囲解除のためのイベントを送信する
        // なぜならアクション実行後にイベントを送信しても
        // BufferChar の座標が変わっていて正しく選択範囲を解除できないため
        if op.is_unmark_operation() {
            pre_selection.iter().cloned().for_each(|c| {
                self.sender.send(ChangeEvent::UnSelectChar(c)).unwrap();
            });
        }

        action(self);

        let post_selection = self.selection();
        if pre_selection != post_selection {
            let leave_selections = pre_selection
                .iter()
                .filter(|c| !post_selection.contains(c))
                .cloned()
                .collect::<Vec<_>>();
            leave_selections.iter().for_each(|c| {
                self.sender.send(ChangeEvent::UnSelectChar(*c)).unwrap();
            });

            let enter_selections = post_selection
                .iter()
                .filter(|c| !pre_selection.contains(c))
                .cloned()
                .collect::<Vec<_>>();
            enter_selections.iter().for_each(|c| {
                self.sender.send(ChangeEvent::SelectChar(*c)).unwrap();
            });
        }
    }

    pub fn operation(&mut self, op: &EditorOperation) {
        self.action_width_selection_update(op, |itself| {
            match op {
                EditorOperation::Undo => {
                    itself.undo();
                    return;
                }
                EditorOperation::Mark => {
                    itself.mark();
                    return;
                }
                EditorOperation::UnMark => {
                    itself.unmark();
                    return;
                }
                _ => (),
            }
            let reverse_actions = BufferApplyer::apply_action(
                &mut itself.buffer,
                &mut itself.main_caret,
                &mut itself.mark,
                op,
                &itself.sender,
            );
            itself.undo_list.push(reverse_actions);

            if op.is_unmark_operation() {
                itself.unmark();
            }
        });
    }

    fn undo(&mut self) {
        if let Some(reverse_action) = self.undo_list.pop() {
            BufferApplyer::apply_reserve_actions(
                &mut self.buffer,
                &mut self.main_caret,
                &mut self.mark,
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
        self.mark = Some(Caret::new_mark(self.main_caret.position, &self.sender));
    }

    pub fn unmark(&mut self) {
        if let Some(current_mark) = self.mark {
            self.sender
                .send(ChangeEvent::RemoveCaret(current_mark))
                .unwrap();
            self.mark = None;
        }
    }

    pub fn to_buffer_string(&self) -> String {
        self.buffer.to_buffer_string()
    }

    pub fn buffer_chars(&self) -> Vec<Vec<BufferChar>> {
        self.buffer
            .lines
            .iter()
            .map(|line| line.chars.clone())
            .collect()
    }

    pub fn main_caret(&self) -> Caret {
        self.main_caret
    }

    pub fn mark_caret(&self) -> Option<Caret> {
        self.mark
    }

    fn selection(&self) -> Vec<BufferChar> {
        let Some(mark) = self.mark else {
            return Vec::new();
        };
        let (from, to) = if self.main_caret < mark {
            (self.main_caret.position, mark.position)
        } else {
            (mark.position, self.main_caret.position)
        };
        if from.is_same_row(&to) {
            self.buffer.lines[from.row].chars[from.col..to.col].to_vec()
        } else {
            let mut result = Vec::new();
            result.extend(self.buffer.lines[from.row].chars[from.col..].iter());
            for row in from.row + 1..to.row {
                result.extend(self.buffer.lines[row].chars.iter());
            }
            result.extend(self.buffer.lines[to.row].chars[..to.col].iter());
            result
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ChangeEvent {
    AddChar(BufferChar),
    MoveChar { from: BufferChar, to: BufferChar },
    RemoveChar(BufferChar),
    SelectChar(BufferChar),
    UnSelectChar(BufferChar),
    AddCaret(Caret),
    MoveCaret { from: Caret, to: Caret },
    RemoveCaret(Caret),
}
