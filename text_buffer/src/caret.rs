use std::sync::mpsc::Sender;

use instant::SystemTime;

use crate::editor::ChangeEvent;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Caret {
    pub row: usize,
    pub col: usize,
    // Ord の評価としての優先度は row, col, unique_key の順番になるのでフィールドの最下位に定義する。
    // これにより同じ位置の Caret が複数ある場合に unique_key で区別できるようになる。
    pub unique_key: u128,
}

impl Caret {
    pub fn new(row: usize, col: usize, sender: &Sender<ChangeEvent>) -> Self {
        let instance = Self {
            row,
            col,
            unique_key: gen_uqnique_key(),
        };
        sender.send(ChangeEvent::AddCaret(instance)).unwrap();
        instance
    }

    pub fn new_without_event(row: usize, col: usize) -> Self {
        Self {
            row,
            col,
            unique_key: gen_uqnique_key(),
        }
    }

    #[inline]
    pub fn move_to(&mut self, row: usize, col: usize, sender: &Sender<ChangeEvent>) {
        if self.row == row && self.col == col {
            return;
        }
        let from = self.clone();
        self.row = row;
        self.col = col;
        let event = ChangeEvent::MoveCaret { from, to: *self };
        sender.send(event).unwrap();
    }

    pub fn to(&mut self, to: &Caret, sender: &Sender<ChangeEvent>) {
        self.move_to(to.row, to.col, sender);
    }
}

// 実用上問題なければいいので精度は ms 単位で十分としておく。
// 自動テストなどで問題が起きれば対応を検討する。
#[inline]
fn gen_uqnique_key() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}
