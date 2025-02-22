use std::sync::{Arc, LazyLock};

use cached::proc_macro::cached;
use font_collector::FontData;
use log::debug;
use rustybuzz::Face;
use text_buffer::editor::CharWidthResolver;
use unicode_width::UnicodeWidthChar;

pub struct CharWidthCalculator {
    faces: Arc<Vec<FontData>>,
}

impl CharWidthCalculator {
    pub fn new(faces: Arc<Vec<FontData>>) -> Self {
        Self { faces }
    }

    pub fn get_width(&self, c: char) -> CharWidth {
        inner_get_width(&self.faces, c)
    }

    pub fn len(&self, text: &str) -> usize {
        text.chars()
            .map(|c| match self.get_width(c) {
                crate::char_width_calcurator::CharWidth::Regular => 1,
                crate::char_width_calcurator::CharWidth::Wide => 2,
            })
            .sum()
    }
}

static SPECIAL_WIDE_CHARS: LazyLock<Vec<char>> = LazyLock::new(|| {
    let mut v = Vec::new();
    v.push('　');
    // 割と雑だが、ギリシャ文字は全角として扱う
    ('Α'..='Ω').for_each(|c| v.push(c));
    ('α'..='ω').for_each(|c| v.push(c));
    v
});

#[cached(key = "char", convert = "{ c }")]
fn inner_get_width(faces: &[FontData], c: char) -> CharWidth {
    debug!("char:{:?}", c);
    if SPECIAL_WIDE_CHARS.contains(&c) {
        debug!("reson:special_wide_chars");
        return CharWidth::Wide;
    }
    if c.is_ascii() {
        debug!("reson:ascii");
        return CharWidth::Regular;
    }
    for face in faces
        .iter()
        .flat_map(|f| Face::from_slice(&f.binary, f.index))
    {
        if let Some(width) = calc_width(c, &face) {
            debug!("reson:calc_width");
            return width;
        }
    }
    debug!("reson:unicode_width");
    match UnicodeWidthChar::width_cjk(c) {
        Some(1) => CharWidth::Regular,
        Some(_) => CharWidth::Wide,
        None => CharWidth::Regular,
    }
}

fn calc_width(c: char, face: &Face) -> Option<CharWidth> {
    if let Some(glyph_id) = face.glyph_index(c) {
        if let Some(rect) = face.glyph_bounding_box(glyph_id) {
            // rect の横幅が face の高さの半分を超える場合は Wide とする
            if face.height() < rect.width() * 2 {
                return Some(CharWidth::Wide);
            }
        }
    }
    debug!("calc_width:None");
    None
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CharWidth {
    Regular,
    Wide,
}

impl CharWidth {
    /// 描画時に左にどれぐらい移動させるか
    pub fn left(&self) -> f32 {
        match self {
            CharWidth::Regular => -0.25,
            CharWidth::Wide => 0.0,
        }
    }

    /// 描画時に右にどれぐらい移動させるか
    pub fn right(&self) -> f32 {
        match self {
            CharWidth::Regular => 0.75,
            CharWidth::Wide => 1.0,
        }
    }

    /// グリフ自体の横幅
    pub fn to_f32(self) -> f32 {
        match self {
            CharWidth::Regular => 0.5,
            CharWidth::Wide => 1.0,
        }
    }
}

impl CharWidthResolver for CharWidthCalculator {
    fn resolve_width(&self, c: char) -> usize {
        match self.get_width(c) {
            CharWidth::Regular => 1,
            CharWidth::Wide => 2,
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use font_collector::FontCollector;

    use super::{CharWidth, CharWidthCalculator};

    const FONT_DATA: &[u8] = include_bytes!("../../fonts/BIZUDMincho-Regular.ttf");
    const EMOJI_FONT_DATA: &[u8] = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");

    #[test]
    fn get_width() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();

        let collector = FontCollector::default();

        let font_binaries = vec![
            collector.convert_font(FONT_DATA.to_vec(), None).unwrap(),
            collector
                .convert_font(EMOJI_FONT_DATA.to_vec(), None)
                .unwrap(),
        ];

        let font_binaries = Arc::new(font_binaries);
        let converter = CharWidthCalculator::new(font_binaries);

        let mut cases = vec![
            // 縦書きでも同じグリフが使われる文字
            ('a', CharWidth::Regular),
            ('あ', CharWidth::Wide),
            ('🐖', CharWidth::Wide),
            ('☺', CharWidth::Wide),
            // 全角スペースは Wide
            ('　', CharWidth::Wide),
        ];
        // 半角アルファベットは CharWidth::Regular
        let mut alpha_cases = ('A'..='z')
            .map(|c| (c, CharWidth::Regular))
            .collect::<Vec<_>>();
        cases.append(&mut alpha_cases);
        // 全角アルファベットは CharWidth::Wide
        let mut zen_alpha_cases = ('Ａ'..='ｚ')
            .map(|c| (c, CharWidth::Wide))
            .collect::<Vec<_>>();
        cases.append(&mut zen_alpha_cases);
        // ギリシャ文字は CharWidth::Wide
        let mut zen_upper_greek_cases = ('Α'..='Ω')
            .map(|c| (c, CharWidth::Wide))
            .collect::<Vec<_>>();
        cases.append(&mut zen_upper_greek_cases);
        let mut zen_lower_greek_cases = ('α'..='ω')
            .map(|c| (c, CharWidth::Wide))
            .collect::<Vec<_>>();
        cases.append(&mut zen_lower_greek_cases);
        for (c, expected) in cases {
            let actual = converter.get_width(c);
            assert_eq!(actual, expected, "char:{}", c);
        }
    }
}
