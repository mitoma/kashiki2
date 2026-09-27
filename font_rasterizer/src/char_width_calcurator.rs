use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};

use font_collector::FontData;
use log::debug;
use phisical_layouter::{CharWidthResolver, PhysicalLayoutMode};
use skrifa::{
    FontRef, MetadataProvider,
    instance::{LocationRef, Size},
};
use unicode_width::UnicodeWidthChar;

pub struct CharWidthCalculator {
    faces: Arc<Vec<FontData>>,
    is_proportional_font: bool,
    widths: RwLock<HashMap<char, CharWidth>>,
}

impl CharWidthCalculator {
    pub fn new(faces: Arc<Vec<FontData>>) -> Self {
        Self::new_with_ascii_override(faces, None)
    }

    pub fn new_with_ascii_override(
        faces: Arc<Vec<FontData>>,
        ascii_override_font: Option<&FontData>,
    ) -> Self {
        let is_proportional_font = detect_proportional_font(&faces)
            || ascii_override_font
                .is_some_and(|font| detect_proportional_font(std::slice::from_ref(font)));
        let mut widths = HashMap::new();
        if let Some(font) = ascii_override_font
            && let Ok(font) = FontRef::from_index(&font.binary, font.index)
        {
            for c in (0u8..=127).map(char::from) {
                if let Some(width) = calc_proportional_width(c, &font) {
                    widths.insert(c, width);
                }
            }
        }
        Self {
            faces,
            is_proportional_font,
            widths: RwLock::new(widths),
        }
    }

    pub fn get_width(&self, c: char) -> CharWidth {
        if let Some(width) = self.widths.read().unwrap().get(&c) {
            return *width;
        }

        let width = inner_get_width(&self.faces, c, self.is_proportional_font);
        self.widths.write().unwrap().insert(c, width);
        width
    }

    pub fn is_proportional_font(&self) -> bool {
        self.is_proportional_font
    }

    pub fn len(&self, text: &str) -> usize {
        text.chars()
            .map(|c| self.get_width(c).to_f32() * 2.0)
            .sum::<f32>()
            .ceil() as usize
    }
}

fn detect_proportional_font(faces: &[FontData]) -> bool {
    const PROBE_CHARS: [char; 5] = ['i', 'M', 'W', '0', ' '];

    for face in faces {
        let Ok(font) = FontRef::from_index(&face.binary, face.index) else {
            continue;
        };
        let glyph_metrics = font.glyph_metrics(Size::unscaled(), LocationRef::default());
        let advances = PROBE_CHARS
            .iter()
            .filter_map(|c| font.charmap().map(*c))
            .filter_map(|glyph_id| glyph_metrics.advance_width(glyph_id))
            .collect::<Vec<_>>();
        if advances.len() != PROBE_CHARS.len() {
            continue;
        }

        let units_per_em = font
            .metrics(Size::unscaled(), LocationRef::default())
            .units_per_em;
        return has_proportional_advances(&advances, units_per_em as f32);
    }
    false
}

fn has_proportional_advances(advances: &[f32], units_per_em: f32) -> bool {
    if advances.is_empty() || !units_per_em.is_finite() || units_per_em <= 0.0 {
        return false;
    }
    let min_advance = advances.iter().copied().fold(f32::INFINITY, f32::min);
    let max_advance = advances.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    max_advance - min_advance > units_per_em * 0.02
}

static SPECIAL_WIDE_CHARS: LazyLock<Vec<char>> = LazyLock::new(|| {
    let mut v = Vec::new();
    v.push('　');
    // 割と雑だが、ギリシャ文字は全角として扱う
    ('Α'..='Ω').for_each(|c| v.push(c));
    ('α'..='ω').for_each(|c| v.push(c));
    v
});

fn inner_get_width(faces: &[FontData], c: char, is_proportional_font: bool) -> CharWidth {
    debug!("char:{:?}", c);
    if SPECIAL_WIDE_CHARS.contains(&c) {
        debug!("reson:special_wide_chars");
        return CharWidth::Wide;
    }
    if is_proportional_font {
        for font in faces
            .iter()
            .flat_map(|f| FontRef::from_index(&f.binary, f.index).ok())
        {
            if let Some(width) = calc_proportional_width(c, &font) {
                return width;
            }
        }
    }
    if c.is_ascii() {
        debug!("reson:ascii");
        return CharWidth::Regular;
    }
    for font in faces
        .iter()
        .flat_map(|f| FontRef::from_index(&f.binary, f.index).ok())
    {
        if let Some(width) = calc_width(c, &font) {
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

fn calc_proportional_width(c: char, font: &FontRef) -> Option<CharWidth> {
    let glyph_id = font.charmap().map(c)?;
    let units_per_em = font
        .metrics(Size::unscaled(), LocationRef::default())
        .units_per_em as f32;
    if units_per_em <= 0.0 {
        return None;
    }
    let advance = font
        .glyph_metrics(Size::unscaled(), LocationRef::default())
        .advance_width(glyph_id)?;
    if !advance.is_finite() || advance < 0.0 {
        return None;
    }
    Some(CharWidth::Proportional(advance / units_per_em))
}

fn calc_width(c: char, font: &FontRef) -> Option<CharWidth> {
    let glyph_id = font.charmap().map(c)?;
    let metrics = font.metrics(Size::unscaled(), LocationRef::default());
    let glyph_metrics = font.glyph_metrics(Size::unscaled(), LocationRef::default());
    if let Some(bounds) = glyph_metrics.bounds(glyph_id) {
        // バウンディングボックスの横幅がフォント高さの半分を超える場合は Wide とする
        let height = metrics.ascent - metrics.descent;
        let width = bounds.x_max - bounds.x_min;
        if height < width * 2.0 {
            return Some(CharWidth::Wide);
        }
    }
    debug!("calc_width:None");
    None
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CharWidth {
    Regular,
    Wide,
    Proportional(f32),
}

impl CharWidth {
    /// 描画時に左にどれぐらい移動させるか
    pub fn left(&self) -> f32 {
        match self {
            CharWidth::Regular => -0.25,
            CharWidth::Wide => 0.0,
            CharWidth::Proportional(width) => (width - 1.0) / 2.0,
        }
    }

    /// 描画時に右にどれぐらい移動させるか
    pub fn right(&self) -> f32 {
        match self {
            CharWidth::Regular => 0.75,
            CharWidth::Wide => 1.0,
            CharWidth::Proportional(width) => (width + 1.0) / 2.0,
        }
    }

    /// グリフ自体の横幅
    pub fn to_f32(self) -> f32 {
        match self {
            CharWidth::Regular => 0.5,
            CharWidth::Wide => 1.0,
            CharWidth::Proportional(width) => width,
        }
    }
}

impl CharWidthResolver for CharWidthCalculator {
    fn resolve_width(&self, c: char) -> usize {
        match self.get_width(c) {
            CharWidth::Regular => 1,
            CharWidth::Wide => 2,
            CharWidth::Proportional(width) => (width * 2.0).round().max(0.0) as usize,
        }
    }

    fn resolve_proportional_width(&self, c: char) -> f32 {
        self.get_width(c).to_f32() * 2.0
    }

    fn layout_mode(&self) -> PhysicalLayoutMode {
        if self.is_proportional_font {
            PhysicalLayoutMode::Proportional
        } else {
            PhysicalLayoutMode::Cell
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use font_collector::FontCollector;

    use super::{CharWidth, CharWidthCalculator, has_proportional_advances};

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

    #[test]
    fn detects_proportional_advances() {
        assert!(!has_proportional_advances(&[600.0, 600.0, 600.0], 1000.0));
        assert!(has_proportional_advances(&[300.0, 600.0, 900.0], 1000.0));
        assert!(!has_proportional_advances(&[], 1000.0));
    }

    #[test]
    fn proportional_width_preserves_advance() {
        let width = CharWidth::Proportional(0.6);
        assert_eq!(width.to_f32(), 0.6);
        assert!((width.left() + width.right() - width.to_f32()).abs() < f32::EPSILON);
    }

    #[test]
    fn ascii_override_font_supplies_ascii_widths() {
        let collector = FontCollector::default();
        let override_font = collector.convert_font(FONT_DATA.to_vec(), None).unwrap();
        let expected = super::FontRef::from_index(&override_font.binary, override_font.index)
            .ok()
            .and_then(|font| super::calc_proportional_width('A', &font))
            .expect("test font should contain an advance for 'A'");

        let calculator = CharWidthCalculator::new_with_ascii_override(
            Arc::new(Vec::new()),
            Some(&override_font),
        );

        assert_eq!(calculator.get_width('A'), expected);
    }
}
