#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CharType {
    Whitespace,
    AsciiDigit,
    Alphabet,
    Hiragana,
    Katakana,
    Kanji,
    Leader,
    Other,
}

impl CharType {
    pub(crate) fn from_char(c: char) -> Self {
        if c.is_whitespace() {
            Self::Whitespace
        } else if c.is_ascii_digit() {
            Self::AsciiDigit
        } else if c.is_ascii_alphabetic() {
            Self::Alphabet
        } else {
            match c {
                'ぁ'..='ん' => Self::Hiragana,
                'ァ'..='ン' => Self::Katakana,
                '一'..='龥' => Self::Kanji,
                '～' | 'ー' => Self::Leader,
                _ => Self::Other,
            }
        }
    }

    pub(crate) fn skip_word(&self, next: &Self) -> bool {
        match (self, next) {
            // 同じ文字種の場合はスキップ
            (left, right) if left == right => true,
            // 空白文字からほかの文字種は区切る
            (Self::Whitespace, _) => false,
            // 他の文字種から空白文字はスキップ
            (_, Self::Whitespace) => true,
            // 平仮名からほかの文字種は伸ばし棒以外は区切る
            (Self::Hiragana, Self::Leader) => true,
            (Self::Hiragana, _) => false,
            // カタカナから平仮名はスキップ
            (Self::Katakana, Self::Hiragana) => true,
            (Self::Katakana, Self::Leader) => true,
            // 漢字から平仮名、カタカナはスキップ
            (Self::Kanji, Self::Hiragana) => true,
            (Self::Kanji, Self::Katakana) => true,
            // 伸ばし棒から平仮名・カタカナから伸ばし棒などはスキップ
            (Self::Leader, Self::Hiragana) => true,
            (Self::Leader, Self::Katakana) => true,

            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn char_type_from_char() {
        assert_eq!(CharType::from_char(' '), CharType::Whitespace);
        assert_eq!(CharType::from_char('a'), CharType::Alphabet);
        assert_eq!(CharType::from_char('1'), CharType::AsciiDigit);
        assert_eq!(CharType::from_char('あ'), CharType::Hiragana);
        assert_eq!(CharType::from_char('ア'), CharType::Katakana);
        assert_eq!(CharType::from_char('一'), CharType::Kanji);
        assert_eq!(CharType::from_char('～'), CharType::Leader);
        assert_eq!(CharType::from_char('ー'), CharType::Leader);
        assert_eq!(CharType::from_char('!'), CharType::Other);
    }
}
