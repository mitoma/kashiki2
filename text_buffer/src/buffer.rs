use std::sync::mpsc::Sender;

use crate::{caret::Caret, editor::ChangeEvent};

pub struct Buffer {
    pub lines: Vec<BufferLine>,
    sender: Sender<ChangeEvent>,
}

impl Buffer {
    pub(crate) fn new(sender: Sender<ChangeEvent>) -> Self {
        Self {
            lines: vec![BufferLine::default()],
            sender,
        }
    }

    pub(crate) fn to_buffer_string(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.to_line_string())
            .collect::<Vec<String>>()
            .join("\n")
    }

    pub(crate) fn insert_string(&mut self, caret: &mut Caret, string: String) {
        let mut iter = string.split("\r\n").flat_map(|line| line.split('\n'));
        let first_line = match iter.next() {
            Some(line) => line,
            None => return,
        };
        first_line.chars().for_each(|c| self.insert_char(caret, c));
        iter.for_each(|line| {
            self.insert_enter(caret);
            line.chars().for_each(|c| self.insert_char(caret, c))
        })
    }

    pub(crate) fn insert_char(&mut self, caret: &mut Caret, c: char) {
        if let Some(line) = self.lines.get_mut(caret.row) {
            line.insert_char(caret.col, c, &self.sender);
            caret.move_to(caret.row, caret.col + 1, &self.sender);
        }
    }

    pub(crate) fn insert_enter(&mut self, caret: &mut Caret) {
        if let Some(line) = self.lines.get_mut(caret.row) {
            if let Some(mut next_line) = line.insert_enter(caret.col) {
                self.lines
                    .iter_mut()
                    .skip(caret.row + 1)
                    .rev()
                    .for_each(|line| line.update_position(line.row_num + 1, &self.sender));
                next_line.update_position(caret.row + 1, &self.sender);
                self.lines.insert(caret.row + 1, next_line);
                caret.move_to(caret.row + 1, 0, &self.sender);
            }
        }
    }

    fn update_position(&mut self) {
        (0..).zip(self.lines.iter_mut()).for_each(|(i, l)| {
            l.update_position(i, &self.sender);
        })
    }

    pub(crate) fn head(&self, caret: &mut Caret) {
        caret.move_to(caret.row, 0, &self.sender);
    }

    pub(crate) fn last(&self, caret: &mut Caret) {
        if let Some(line) = self.lines.get(caret.row) {
            caret.move_to(caret.row, line.chars.len(), &self.sender);
        }
    }

    pub(crate) fn back(&mut self, caret: &mut Caret) {
        match (self.is_line_head(caret), self.is_buffer_head(caret)) {
            (true, true) => {}
            (true, false) => {
                self.previous(caret);
                self.last(caret);
            }
            (false, true) | (false, false) => caret.move_to(caret.row, caret.col - 1, &self.sender),
        }
    }

    pub(crate) fn forward(&mut self, caret: &mut Caret) {
        match (self.is_line_last(caret), self.is_buffer_last(caret)) {
            (true, true) => {}
            (true, false) => {
                self.next(caret);
                self.head(caret);
            }
            (false, true) | (false, false) => caret.move_to(caret.row, caret.col + 1, &self.sender),
        }
    }

    pub(crate) fn previous(&mut self, caret: &mut Caret) {
        if !self.is_buffer_head(caret) {
            caret.move_to(caret.row - 1, caret.col, &self.sender);
            if self.is_line_last(caret) {
                // 前行が短い場合に Caret 位置を調整
                self.last(caret)
            }
        }
    }

    pub(crate) fn next(&self, caret: &mut Caret) {
        if !self.is_buffer_last(caret) {
            caret.move_to(caret.row + 1, caret.col, &self.sender);
            if self.is_line_last(caret) {
                // 次行が短い場合に Caret 位置を調整
                self.last(caret)
            }
        }
    }

    pub(crate) fn buffer_head(&self, caret: &mut Caret) {
        caret.move_to(0, 0, &self.sender);
    }

    pub(crate) fn buffer_last(&self, caret: &mut Caret) {
        if let Some(last_line) = self.lines.last() {
            caret.move_to(last_line.row_num, last_line.chars.len(), &self.sender);
        }
    }

    fn is_buffer_head(&self, caret: &Caret) -> bool {
        caret.row == 0
    }

    fn is_buffer_last(&self, caret: &Caret) -> bool {
        caret.row == self.lines.len() - 1
    }

    fn is_line_head(&self, caret: &Caret) -> bool {
        caret.col == 0
    }

    fn is_line_last(&self, caret: &Caret) -> bool {
        if let Some(line_length) = self.lines.get(caret.row).map(|line| line.chars.len()) {
            caret.col >= line_length
        } else {
            false
        }
    }

    pub(crate) fn backspace(&mut self, caret: &mut Caret) -> RemovedChar {
        if self.is_buffer_head(caret) && self.is_line_head(caret) {
            RemovedChar::None
        } else {
            self.back(caret);
            self.delete(caret)
        }
    }

    pub(crate) fn delete(&mut self, caret: &Caret) -> RemovedChar {
        if self.is_line_last(caret) {
            if !self.is_buffer_last(caret) {
                let next_line = self.lines.remove(caret.row + 1);
                let current_line = self.lines.get_mut(caret.row).unwrap();
                current_line.join(next_line, &self.sender);
                self.update_position();
                RemovedChar::Enter
            } else {
                RemovedChar::None
            }
        } else if let Some(line) = self.lines.get_mut(caret.row) {
            line.remove_char(caret.col, &self.sender)
        } else {
            RemovedChar::None
        }
    }
}

#[derive(Default)]
pub struct BufferLine {
    row_num: usize,
    chars: Vec<BufferChar>,
}

impl BufferLine {
    fn from_chars(chars: Vec<BufferChar>) -> BufferLine {
        BufferLine { row_num: 0, chars }
    }

    fn update_position(&mut self, row_num: usize, sender: &Sender<ChangeEvent>) {
        self.row_num = row_num;
        (0..).zip(self.chars.iter_mut()).for_each(|(i, c)| {
            c.update_position(row_num, i, sender);
        })
    }

    fn to_line_string(&self) -> String {
        self.chars.iter().map(|c| c.c).collect()
    }

    fn insert_char(&mut self, col: usize, c: char, sender: &Sender<ChangeEvent>) {
        self.chars
            .iter_mut()
            .skip(col)
            .rev()
            .for_each(|c| c.update_position(self.row_num, c.col + 1, sender));
        self.chars
            .insert(col, BufferChar::new(self.row_num, col, c, sender))
    }

    fn insert_enter(&mut self, col: usize) -> Option<BufferLine> {
        match self.chars.len() {
            len if len == col => {
                let line = BufferLine {
                    row_num: self.row_num + 1,
                    ..BufferLine::default()
                };
                Some(line)
            }
            len if len > col => {
                let mut line = BufferLine::from_chars(self.chars.split_off(col));
                line.row_num = self.row_num + 1;
                Some(line)
            }
            _ => None,
        }
    }

    fn remove_char(&mut self, col: usize, sender: &Sender<ChangeEvent>) -> RemovedChar {
        let removed = self.chars.remove(col);
        sender.send(ChangeEvent::RemoveChar(removed)).unwrap();
        self.chars
            .iter_mut()
            .skip(col)
            .for_each(|c| c.update_position(self.row_num, c.col - 1, sender));
        RemovedChar::Char(removed.c)
    }

    fn join(&mut self, line: BufferLine, sender: &Sender<ChangeEvent>) {
        let current_len = self.chars.len();
        line.chars
            .into_iter()
            .map(|mut c| {
                c.update_position(self.row_num, current_len + c.col, sender);
                c
            })
            .for_each(|c| self.chars.push(c))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct BufferChar {
    pub row: usize,
    pub col: usize,
    pub c: char,
}

impl BufferChar {
    fn new(row: usize, col: usize, c: char, sender: &Sender<ChangeEvent>) -> Self {
        let instance = Self { row, col, c };
        sender.send(ChangeEvent::AddChar(instance)).unwrap();
        instance
    }

    fn update_position(&mut self, row: usize, col: usize, sender: &Sender<ChangeEvent>) {
        if self.row == row && self.col == col {
            return;
        }
        let from = self.clone();
        self.row = row;
        self.col = col;
        let event = ChangeEvent::MoveChar { from, to: *self };
        sender.send(event).unwrap();
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum RemovedChar {
    Char(char),
    Enter,
    None,
}

#[cfg(test)]
mod tests {

    use std::sync::mpsc::channel;

    use crate::editor::ChangeEvent;

    use super::*;

    #[test]
    fn buffer() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let caret = &mut Caret::new(0, 0, &tx);
        let mut sut = Buffer::new(tx.clone());
        assert_eq!(sut.to_buffer_string(), "");
        sut.insert_char(caret, '山');
        assert_eq!(sut.to_buffer_string(), "山");
        assert_eq!(caret.row, 0);
        assert_eq!(caret.col, 1);
        sut.insert_char(caret, '本');
        assert_eq!(sut.to_buffer_string(), "山本");
        assert_eq!(caret.row, 0);
        assert_eq!(caret.col, 2);
        sut.insert_enter(caret);
        assert_eq!(sut.to_buffer_string(), "山本\n");
        assert_eq!(caret.row, 1);
        assert_eq!(caret.col, 0);
        sut.insert_enter(caret);
        assert_eq!(sut.to_buffer_string(), "山本\n\n");
        assert_eq!(caret.row, 2);
        assert_eq!(caret.col, 0);
        sut.insert_enter(&mut Caret::new(100, 100, &tx));
        assert_eq!(sut.to_buffer_string(), "山本\n\n");
    }

    #[test]
    fn buffer_insert_string() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let caret = &mut Caret::new(0, 0, &tx);
        let mut sut = Buffer::new(tx);
        sut.insert_string(caret, "東京は\n今日もいい天気\nだった。".to_string());
        assert_eq!(sut.to_buffer_string(), "東京は\n今日もいい天気\nだった。");
        assert_eq!(caret.row, 2);
        assert_eq!(caret.col, 4);
    }

    #[test]
    fn buffer_position_check() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let mut sut = Buffer::new(tx.clone());
        sut.insert_string(
            &mut Caret::new(0, 0, &tx),
            "あいうえお\nかきくけこ\nさしすせそそ".to_string(),
        );
        // buffer head
        assert!(sut.is_buffer_head(&Caret::new(0, 0, &tx)));
        assert!(sut.is_buffer_head(&Caret::new(0, 4, &tx)));
        assert!(!sut.is_buffer_head(&Caret::new(1, 0, &tx)));

        // buffer last
        assert!(sut.is_buffer_last(&Caret::new(2, 0, &tx)));
        assert!(sut.is_buffer_last(&Caret::new(2, 4, &tx)));
        assert!(!sut.is_buffer_last(&Caret::new(0, 0, &tx)));

        // line head
        assert!(sut.is_line_head(&Caret::new(0, 0, &tx)));
        assert!(sut.is_line_head(&Caret::new(2, 0, &tx)));
        assert!(!sut.is_line_head(&Caret::new(1, 3, &tx)));

        // line last
        assert!(sut.is_line_last(&Caret::new(0, 5, &tx)));
        assert!(sut.is_line_last(&Caret::new(2, 6, &tx)));
        assert!(!sut.is_line_last(&Caret::new(2, 5, &tx)));
    }

    #[test]
    fn buffer_move() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let mut sut = Buffer::new(tx.clone());
        let caret = &mut Caret::new(0, 0, &tx);
        sut.insert_string(caret, "あいうえお\nきかくけここ\nさしすせそ".to_string());

        // forward
        caret.move_to(0, 0, &tx);
        sut.forward(caret);
        assert_eq!(caret, &Caret::new(0, 1, &tx));

        caret.move_to(0, 4, &tx);
        sut.forward(caret);
        assert_eq!(caret, &Caret::new(0, 5, &tx));

        caret.move_to(0, 5, &tx);
        sut.forward(caret);
        assert_eq!(caret, &Caret::new(1, 0, &tx));

        caret.move_to(2, 5, &tx);
        sut.forward(caret);
        assert_eq!(caret, &Caret::new(2, 5, &tx));

        // back
        caret.move_to(0, 3, &tx);
        sut.back(caret);
        assert_eq!(caret, &Caret::new(0, 2, &tx));

        caret.move_to(0, 0, &tx);
        sut.back(caret);
        assert_eq!(caret, &Caret::new(0, 0, &tx));

        caret.move_to(2, 0, &tx);
        sut.back(caret);
        assert_eq!(caret, &Caret::new(1, 6, &tx));

        // previous
        caret.move_to(1, 3, &tx);
        sut.previous(caret);
        assert_eq!(caret, &Caret::new(0, 3, &tx));

        caret.move_to(1, 5, &tx);
        sut.previous(caret);
        assert_eq!(caret, &Caret::new(0, 5, &tx));

        caret.move_to(2, 4, &tx);
        sut.previous(caret);
        assert_eq!(caret, &Caret::new(1, 4, &tx));

        // next
        caret.move_to(0, 3, &tx);
        sut.next(caret);
        assert_eq!(caret, &Caret::new(1, 3, &tx));

        caret.move_to(1, 6, &tx);
        sut.next(caret);
        assert_eq!(caret, &Caret::new(2, 5, &tx));

        caret.move_to(2, 5, &tx);
        sut.next(caret);
        assert_eq!(caret, &Caret::new(2, 5, &tx));
    }

    #[test]
    fn buffer_backspace() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let mut sut = Buffer::new(tx.clone());
        sut.insert_string(
            &mut Caret::new(0, 0, &tx),
            "あいうえお\nかきくけこ\nさしすせそ".to_string(),
        );
        assert_eq!(
            sut.backspace(&mut Caret::new(1, 3, &tx)),
            RemovedChar::Char('く')
        );
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきけこ\nさしすせそ".to_string()
        );
        assert_eq!(
            sut.backspace(&mut Caret::new(1, 4, &tx)),
            RemovedChar::Char('こ')
        );
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきけ\nさしすせそ".to_string()
        );
        assert_eq!(
            sut.backspace(&mut Caret::new(2, 0, &tx)),
            RemovedChar::Enter
        );
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきけさしすせそ".to_string()
        );
    }

    #[test]
    fn buffer_delete() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let mut sut = Buffer::new(tx.clone());
        sut.insert_string(
            &mut Caret::new(0, 0, &tx),
            "あいうえお\nかきくけこ\nさしすせそ".to_string(),
        );
        assert_eq!(sut.delete(&Caret::new(1, 3, &tx)), RemovedChar::Char('け'));
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきくこ\nさしすせそ".to_string()
        );
        assert_eq!(sut.delete(&Caret::new(1, 3, &tx)), RemovedChar::Char('こ'));
        assert_eq!(sut.delete(&Caret::new(1, 3, &tx)), RemovedChar::Enter);
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきくさしすせそ".to_string()
        );
        assert_eq!(sut.delete(&Caret::new(1, 7, &tx)), RemovedChar::Char('そ'));
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきくさしすせ".to_string()
        );
        assert_eq!(sut.delete(&Caret::new(1, 7, &tx)), RemovedChar::None);
        assert_eq!(
            sut.to_buffer_string(),
            "あいうえお\nかきくさしすせ".to_string()
        );
    }

    #[test]
    fn buffer_line_insert_remove() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let mut sut = BufferLine::default();
        assert_eq!(sut.to_line_string(), "");
        sut.insert_char(0, '鉄', &tx);
        sut.insert_char(1, 'ン', &tx);
        assert_eq!(sut.to_line_string(), "鉄ン");
        sut.insert_char(1, '鍋', &tx);
        sut.insert_char(2, 'の', &tx);
        sut.insert_char(3, 'ャ', &tx);
        sut.insert_char(3, 'ジ', &tx);
        assert_eq!(sut.to_line_string(), "鉄鍋のジャン");
        assert_eq!(sut.remove_char(4, &tx), RemovedChar::Char('ャ'));
        assert_eq!(sut.remove_char(3, &tx), RemovedChar::Char('ジ'));
        assert_eq!(sut.to_line_string(), "鉄鍋のン");
    }

    #[test]
    fn buffer_line_enter_join() {
        let (tx, _rx) = channel::<ChangeEvent>();
        let mut sut = BufferLine::default();
        assert_eq!(sut.to_line_string(), "");
        sut.insert_char(0, '花', &tx);
        sut.insert_char(1, '鳥', &tx);
        sut.insert_char(2, '風', &tx);
        sut.insert_char(3, '月', &tx);
        if let Some(result) = sut.insert_enter(2) {
            assert_eq!(sut.to_line_string(), "花鳥");
            assert_eq!(result.to_line_string(), "風月");
            sut.join(result, &tx);
        } else {
            assert!(false);
        }
        assert_eq!(sut.to_line_string(), "花鳥風月");
        if let Some(result) = sut.insert_enter(4) {
            assert_eq!(sut.to_line_string(), "花鳥風月");
            assert_eq!(result.to_line_string(), "");
        } else {
            assert!(false);
        }
        if sut.insert_enter(5).is_some() {
            assert!(false);
        } else {
            assert!(true);
        }
    }

    #[test]
    fn event_test() {
        let (tx, rx) = channel::<ChangeEvent>();
        let mut caret = Caret::new(0, 0, &tx);
        let mut sut = Buffer::new(tx);

        sut.insert_char(&mut caret, 'あ');
        {
            let events: Vec<ChangeEvent> = rx.try_iter().collect();
            #[rustfmt::skip]
            assert_eq!(
                events,
                vec![
                    ChangeEvent::AddCaret(Caret { row: 0, col: 0 }),
                    ChangeEvent::AddChar(BufferChar { row: 0, col: 0, c: 'あ' }),
                    ChangeEvent::MoveCaret { from: Caret { row: 0, col: 0 }, to: Caret { row: 0, col: 1 } }
                ]
            );
        }
        sut.insert_char(&mut caret, 'い');
        {
            let events: Vec<ChangeEvent> = rx.try_iter().collect();
            #[rustfmt::skip]
            assert_eq!(
                events,
                vec![
                    ChangeEvent::AddChar(BufferChar { row: 0, col: 1, c: 'い' }),
                    ChangeEvent::MoveCaret { from: Caret { row: 0, col: 1 }, to: Caret { row: 0, col: 2 } }
                ]
            );
        }
        sut.insert_enter(&mut caret);
        {
            let events: Vec<ChangeEvent> = rx.try_iter().collect();
            #[rustfmt::skip]
            assert_eq!(
                events,
                vec![
                    ChangeEvent::MoveCaret { from: Caret { row: 0, col: 2 }, to: Caret { row: 1, col: 0 } }
                ]
            );
            assert_eq!(caret, Caret::new_without_event(1, 0));
        }
        sut.insert_char(&mut caret, 'え');
        {
            let events: Vec<ChangeEvent> = rx.try_iter().collect();
            #[rustfmt::skip]
            assert_eq!(
                events,
                vec![
                    ChangeEvent::AddChar(BufferChar { row: 1, col: 0, c: 'え' }),
                    ChangeEvent::MoveCaret { from: Caret { row: 1, col: 0 }, to: Caret { row: 1, col: 1 } }
                ]
            );
        }
        sut.insert_char(&mut caret, 'お');
        {
            let events: Vec<ChangeEvent> = rx.try_iter().collect();
            #[rustfmt::skip]
            assert_eq!(
                events,
                vec![
                    ChangeEvent::AddChar(BufferChar { row: 1, col: 1, c: 'お' }),
                    ChangeEvent::MoveCaret { from: Caret { row: 1, col: 1 }, to: Caret { row: 1, col: 2 } }
                ]
            );
        }
    }

    #[test]
    fn event_buffer() {
        let (tx, rx) = channel::<ChangeEvent>();
        let mut caret = Caret::new(0, 0, &tx);
        let mut sut = Buffer::new(tx);

        sut.insert_string(&mut caret, "あいうえお\nかき\nくけ".into());
        sut.buffer_head(&mut caret);
        sut.forward(&mut caret);
        sut.forward(&mut caret);
        let _ = rx.try_iter().collect::<Vec<_>>();
        sut.insert_enter(&mut caret);
        {
            let events: Vec<ChangeEvent> = rx.try_iter().collect();
            #[rustfmt::skip]
            assert_eq!(
                events,
                vec![
                    ChangeEvent::MoveChar { from: BufferChar { row: 2, col: 0, c: 'く' }, to: BufferChar { row: 3, col: 0, c: 'く' } },
                    ChangeEvent::MoveChar { from: BufferChar { row: 2, col: 1, c: 'け' }, to: BufferChar { row: 3, col: 1, c: 'け' } },
                    ChangeEvent::MoveChar { from: BufferChar { row: 1, col: 0, c: 'か' }, to: BufferChar { row: 2, col: 0, c: 'か' } },
                    ChangeEvent::MoveChar { from: BufferChar { row: 1, col: 1, c: 'き' }, to: BufferChar { row: 2, col: 1, c: 'き' } },
                    // 先に以降の行を逆順で移動してから、改行対象の行を動かす
                    ChangeEvent::MoveChar { from: BufferChar { row: 0, col: 2, c: 'う' }, to: BufferChar { row: 1, col: 0, c: 'う' } },
                    ChangeEvent::MoveChar { from: BufferChar { row: 0, col: 3, c: 'え' }, to: BufferChar { row: 1, col: 1, c: 'え' } },
                    ChangeEvent::MoveChar { from: BufferChar { row: 0, col: 4, c: 'お' }, to: BufferChar { row: 1, col: 2, c: 'お' } },
                    ChangeEvent::MoveCaret { from: Caret { row: 0, col: 2 }, to: Caret { row: 1, col: 0 } },
                ]
            );
        }
    }

    #[test]
    fn event_line_add() {
        let (tx, rx) = channel::<ChangeEvent>();
        let mut caret = Caret::new(0, 0, &tx);
        let mut sut = Buffer::new(tx);

        sut.insert_string(&mut caret, "あいうえお".into());
        sut.head(&mut caret);
        sut.forward(&mut caret);
        let _ = rx.try_iter().collect::<Vec<_>>();
        sut.insert_char(&mut caret, 'A');
        {
            let events: Vec<ChangeEvent> = rx.try_iter().collect();
            #[rustfmt::skip]
            assert_eq!(
                events,
                vec![
                    ChangeEvent::MoveChar { from: BufferChar { row: 0, col: 4, c: 'お' }, to: BufferChar { row: 0, col: 5, c: 'お' } },
                    ChangeEvent::MoveChar { from: BufferChar { row: 0, col: 3, c: 'え' }, to: BufferChar { row: 0, col: 4, c: 'え' } },
                    ChangeEvent::MoveChar { from: BufferChar { row: 0, col: 2, c: 'う' }, to: BufferChar { row: 0, col: 3, c: 'う' } },
                    ChangeEvent::MoveChar { from: BufferChar { row: 0, col: 1, c: 'い' }, to: BufferChar { row: 0, col: 2, c: 'い' } },
                    ChangeEvent::AddChar(BufferChar { row: 0, col: 1, c: 'A' }),
                    ChangeEvent::MoveCaret { from: Caret { row: 0, col: 1 }, to: Caret { row: 0, col: 2 } },
                ]
            );
        }
    }

    #[test]
    fn event_line_delete() {
        let (tx, rx) = channel::<ChangeEvent>();
        let mut caret = Caret::new(0, 0, &tx);
        let mut sut = Buffer::new(tx);

        sut.insert_string(&mut caret, "あいうえお".into());
        sut.back(&mut caret);
        sut.back(&mut caret);
        let _ = rx.try_iter().collect::<Vec<_>>();
        sut.backspace(&mut caret);
        //rx.try_iter().for_each(|e| println!("{:?}", e));
        {
            let events: Vec<ChangeEvent> = rx.try_iter().collect();
            #[rustfmt::skip]
            assert_eq!(
                events,
                vec![
                    ChangeEvent::MoveCaret { from: Caret { row: 0, col: 3 }, to: Caret { row: 0, col: 2 } },
                    ChangeEvent::RemoveChar(BufferChar { row: 0, col: 2, c: 'う' }),
                    ChangeEvent::MoveChar { from: BufferChar { row: 0, col: 3, c: 'え' }, to: BufferChar { row: 0, col: 2, c: 'え' } },
                    ChangeEvent::MoveChar { from: BufferChar { row: 0, col: 4, c: 'お' }, to: BufferChar { row: 0, col: 3, c: 'お' } },
                ]
            );
        }
    }
}
