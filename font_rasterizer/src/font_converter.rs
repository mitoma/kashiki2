use std::sync::Arc;

use font_collector::FontData;
use harfrust::{Direction, FontRef as HarfrustFontRef, ShapeOptions, ShaperData, UnicodeBuffer};
use log::info;
use skrifa::{
    FontRef as SkrifaFontRef, GlyphId, MetadataProvider,
    instance::{LocationRef, Size},
    outline::DrawSettings,
};
use ttf_overlap_remover::OverlapRemoveOutlineBuilder;

use crate::{
    char_width_calcurator::CharWidth,
    debug_mode::DEBUG_FLAGS,
    errors::FontRasterizerError,
    vector_vertex::{CoordinateSystem, VectorVertex, VectorVertexBuilder, VertexBuilderOptions},
};

pub(crate) struct FontFaceData<'a> {
    harfrust: HarfrustFontRef<'a>,
    skrifa: SkrifaFontRef<'a>,
    location: skrifa::instance::Location,
    remove_overlap: bool,
}

pub(crate) struct FontVertexConverter {
    fonts: Arc<Vec<FontData>>,
    ascii_override_font: Option<FontData>,
    #[cfg(all(feature = "cache", not(target_arch = "wasm32")))]
    cache: Option<crate::glyph_cache::GlyphCache>,
}

impl FontVertexConverter {
    pub(crate) fn new(fonts: Arc<Vec<FontData>>, ascii_override_font: Option<FontData>) -> Self {
        #[cfg(all(feature = "cache", not(target_arch = "wasm32")))]
        let cache = {
            let fonts = if let Some(ascii_override_font) = &ascii_override_font {
                let mut v = vec![ascii_override_font.clone()];
                v.append(&mut (*fonts).clone());
                Arc::new(v)
            } else {
                fonts.clone()
            };
            crate::glyph_cache::GlyphCache::open(&fonts)
        };
        Self {
            fonts,
            ascii_override_font,
            #[cfg(all(feature = "cache", not(target_arch = "wasm32")))]
            cache,
        }
    }

    fn is_remove_outline_fontname(fontname: &str) -> bool {
        // Noto 系の文字を全部オーバーラップ除去対象にしてみる
        // TODO おそらくひたすら遅くなるはずなのでオーバーラップ除去処理結果をキャッシュする実装を追加したい
        //["Noto Emoji Regular"].contains(&fontname)
        fontname.contains("Noto")
    }

    fn font_data_to_font_face(font_data: &'_ FontData) -> Option<FontFaceData<'_>> {
        let harfrust = HarfrustFontRef::from_index(&font_data.binary, font_data.index).ok()?;
        let skrifa = SkrifaFontRef::from_index(&font_data.binary, font_data.index).ok()?;

        // variable font の際に wght を Noto 系の Regular で指定されがちな 400 に指定する
        // なぜなら、デフォルトだと 100 になってしまっておりやたら細くなってしまうからだ
        // TODO 固定の指定ではなくて柔軟に、wght 以外のタグに指定できるようにしていく必要がある
        let axes = skrifa.axes();
        let location = if axes.iter().any(|a| a.tag() == skrifa::Tag::new(b"wght")) {
            for axis in axes.iter() {
                info!("variation: {}={}", axis.tag(), axis.default_value());
            }
            info!("set weight");
            axes.location([("wght", 400.0f32)])
        } else {
            info!("not variable");
            axes.location::<&[(&str, f32)]>(&[])
        };

        Some(FontFaceData {
            harfrust,
            skrifa,
            location,
            remove_overlap: Self::is_remove_outline_fontname(&font_data.font_name),
        })
    }

    fn font_faces(&'_ self) -> Vec<FontFaceData<'_>> {
        self.fonts
            .iter()
            .filter_map(Self::font_data_to_font_face)
            .collect::<Vec<_>>()
    }

    fn ascii_override_font_face(&'_ self) -> Option<FontFaceData<'_>> {
        self.ascii_override_font
            .as_ref()
            .and_then(Self::font_data_to_font_face)
    }

    fn glyph_ids_for_font_face(ff: &FontFaceData, c: char) -> Option<CharGlyphIds> {
        let horizontal_glyph_id = ff.skrifa.charmap().map(c)?;
        let mut buf = UnicodeBuffer::new();
        buf.set_direction(Direction::TopToBottom);
        buf.add(c, 0);
        let shaper_data = ShaperData::new(&ff.harfrust);
        let shaper = shaper_data.shaper(&ff.harfrust).build();
        let vertical_glyph_buffer = shaper.shape(buf, ShapeOptions::default());
        let vertical_glyph_id = GlyphId::new(vertical_glyph_buffer.glyph_infos()[0].glyph_id);
        let vertical_glyph_id = if horizontal_glyph_id == vertical_glyph_id {
            None
        } else {
            Some(vertical_glyph_id)
        };
        Some(CharGlyphIds {
            horizontal_glyph_id,
            vertical_glyph_id,
        })
    }

    fn get_face_and_glyph_ids(
        &'_ self,
        c: char,
    ) -> Result<(FontFaceData<'_>, CharGlyphIds), FontRasterizerError> {
        if c.is_ascii()
            && let Some(ff) = self.ascii_override_font_face()
            && let Some(ids) = Self::glyph_ids_for_font_face(&ff, c)
        {
            return Ok((ff, ids));
        }

        for ff in self.font_faces() {
            if let Some(ids) = Self::glyph_ids_for_font_face(&ff, c) {
                return Ok((ff, ids));
            }
        }
        Err(FontRasterizerError::GlyphNotFound(c))
    }

    pub(crate) fn convert(
        &self,
        c: char,
        width: CharWidth,
    ) -> Result<GlyphVertex, FontRasterizerError> {
        // キャッシュヒット時はそのまま返す
        #[cfg(all(feature = "cache", not(target_arch = "wasm32")))]
        if let Some(cache) = &self.cache
            && let Some(glyph) = cache.get(c, width)
        {
            return Ok(glyph);
        }

        let result = self.convert_inner(c, width)?;

        // コンバート結果をキャッシュに保存
        #[cfg(all(feature = "cache", not(target_arch = "wasm32")))]
        if let Some(cache) = &self.cache {
            cache.set(&result, width);
        }

        Ok(result)
    }

    fn convert_inner(&self, c: char, width: CharWidth) -> Result<GlyphVertex, FontRasterizerError> {
        let (
            ff,
            CharGlyphIds {
                horizontal_glyph_id,
                vertical_glyph_id,
            },
        ) = self.get_face_and_glyph_ids(c)?;
        let h_vertex = GlyphVertexBuilder::new().build(horizontal_glyph_id, width, &ff)?;
        let v_vertex = vertical_glyph_id.and_then(|vertical_glyph_id| {
            GlyphVertexBuilder::new()
                .build(vertical_glyph_id, width, &ff)
                .ok()
        });
        Ok(GlyphVertex {
            c,
            h_vertex,
            v_vertex,
        })
    }
}

struct CharGlyphIds {
    horizontal_glyph_id: GlyphId,
    vertical_glyph_id: Option<GlyphId>,
}

#[derive(Debug)]
pub(crate) struct GlyphVertex {
    pub(crate) c: char,
    pub(crate) h_vertex: VectorVertex,
    pub(crate) v_vertex: Option<VectorVertex>,
}

pub struct GlyphVertexBuilder {}

impl GlyphVertexBuilder {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) fn build(
        self,
        glyph_id: GlyphId,
        _width: CharWidth,
        ff: &FontFaceData,
    ) -> Result<VectorVertex, FontRasterizerError> {
        let builder = VectorVertexBuilder::new();
        let location_ref = LocationRef::from(&ff.location);

        let metrics = ff.skrifa.metrics(Size::unscaled(), location_ref);
        let glyph_metrics = ff.skrifa.glyph_metrics(Size::unscaled(), location_ref);

        let rect_em = metrics.units_per_em as f32;
        let center_x = glyph_metrics.advance_width(glyph_id).unwrap_or(0.0) / 2.0;
        let center_y = metrics
            .cap_height
            .or(metrics.x_height)
            .unwrap_or(rect_em * 0.8)
            / 2.0;

        let mut builder = builder.with_options(VertexBuilderOptions::new(
            [center_x, center_y],
            rect_em,
            CoordinateSystem::Font,
            None,
        ));

        let draw_settings = DrawSettings::unhinted(Size::unscaled(), location_ref);
        let outlines = ff.skrifa.outline_glyphs();
        let outline_glyph = outlines
            .get(glyph_id)
            .ok_or(FontRasterizerError::NoOutlineGlyph(glyph_id))?;

        if ff.remove_overlap {
            let mut overlap_builder = OverlapRemoveOutlineBuilder::default();
            outline_glyph
                .draw(draw_settings, &mut overlap_builder)
                .map_err(|_| FontRasterizerError::NoOutlineGlyph(glyph_id))?;
            overlap_builder.outline(&mut builder);
        } else {
            outline_glyph
                .draw(draw_settings, &mut builder)
                .map_err(|_| FontRasterizerError::NoOutlineGlyph(glyph_id))?;
        }

        if DEBUG_FLAGS.show_glyph_outline {
            if let Some(global) = metrics.bounds {
                builder.move_to(global.x_min, global.y_min);
                builder.line_to(global.x_max, global.y_min);
                builder.line_to(global.x_max, global.y_max);
                builder.line_to(global.x_min, global.y_max);
                builder.line_to(global.x_min, global.y_min);
            }
            if let Some(rect) = glyph_metrics.bounds(glyph_id) {
                builder.move_to(rect.x_min, rect.y_min);
                builder.line_to(rect.x_max, rect.y_min);
                builder.line_to(rect.x_max, rect.y_max);
                builder.line_to(rect.x_min, rect.y_max);
                builder.line_to(rect.x_min, rect.y_min);
            }
            let x = center_x;
            let y = center_y;
            builder.move_to(x - 100.0, y);
            builder.line_to(x, y + 100.0);
            builder.line_to(x + 100.0, y);
            builder.line_to(x, y - 100.0);
            builder.line_to(x - 100.0, y);
        }

        Ok(builder.build())
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use font_collector::FontCollector;

    use super::FontVertexConverter;

    const FONT_DATA: &[u8] = include_bytes!("../../fonts/BIZUDMincho-Regular.ttf");
    const EMOJI_FONT_DATA: &[u8] = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");

    #[test]
    fn get_char_glyph_ids_test() {
        let collector = FontCollector::default();
        let font_binaries = vec![
            collector.convert_font(FONT_DATA.to_vec(), None).unwrap(),
            collector
                .convert_font(EMOJI_FONT_DATA.to_vec(), None)
                .unwrap(),
        ];
        let converter = FontVertexConverter::new(Arc::new(font_binaries), None);

        let cases = vec![
            // 縦書きでも同じグリフが使われる文字
            ('a', false),
            ('あ', false),
            ('🐖', false),
            // 縦書きでは別のグリフが使われる文字
            ('。', true),
            ('「', true),
            ('ー', true),
        ];
        for (c, expected) in cases {
            let (_, ids) = converter
                .get_face_and_glyph_ids(c)
                .expect("get char glyph ids");
            assert_eq!(ids.vertical_glyph_id.is_some(), expected);
        }
    }

    #[test]
    fn font_info_test() {
        use skrifa::{
            FontRef, MetadataProvider,
            instance::{LocationRef, Size},
        };
        let fonts = [
            FontRef::new(FONT_DATA).expect("font from slice"),
            FontRef::new(EMOJI_FONT_DATA).expect("font from slice"),
        ];

        for font in fonts.iter() {
            println!("-----------------");
            let metrics = font.metrics(Size::unscaled(), LocationRef::default());
            if let Some(bounds) = metrics.bounds {
                println!(
                    "global:{:?}, width:{}, height:{}",
                    bounds,
                    bounds.x_max - bounds.x_min,
                    bounds.y_max - bounds.y_min,
                );
            }
            println!(
                "em:{:?}, origin_rect_em: {}, new_rect_em: {}",
                metrics.units_per_em,
                (metrics.units_per_em as f32 / 1024.0).sqrt(),
                (metrics.units_per_em as f32).sqrt()
            );
        }
    }
}
