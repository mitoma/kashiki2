use std::sync::Arc;

use font_collector::FontData;
use rustybuzz::{shape, ttf_parser::GlyphId, Direction, Face, UnicodeBuffer};

use crate::{
    char_width_calcurator::CharWidth,
    debug_mode::DEBUG_FLAGS,
    errors::FontRasterizerError,
    vector_vertex::{VectorVertex, VectorVertexBuilder, VertexBuilderOptions},
};

pub(crate) struct FontVertexConverter {
    fonts: Arc<Vec<FontData>>,
}

impl FontVertexConverter {
    pub(crate) fn new(fonts: Arc<Vec<FontData>>) -> Self {
        Self { fonts }
    }

    fn faces(&self) -> Vec<Face> {
        self.fonts
            .iter()
            .flat_map(|f| Face::from_slice(&f.binary, f.index))
            .collect::<Vec<Face>>()
    }

    fn get_face_and_glyph_ids(&self, c: char) -> Result<(Face, CharGlyphIds), FontRasterizerError> {
        let mut buf = UnicodeBuffer::new();
        buf.set_direction(Direction::TopToBottom);
        buf.add(c, 0);
        for face in self.faces().into_iter() {
            if let Some(horizontal_glyph_id) = face.glyph_index(c) {
                let vertical_glyph_buffer = shape(&face, &[], buf);
                let vertical_glyph_id =
                    GlyphId(vertical_glyph_buffer.glyph_infos()[0].glyph_id as u16);
                let vertical_glyph_id = if horizontal_glyph_id == vertical_glyph_id {
                    None
                } else {
                    Some(vertical_glyph_id)
                };
                return Ok((
                    face,
                    CharGlyphIds {
                        horizontal_glyph_id,
                        vertical_glyph_id,
                    },
                ));
            }
        }
        Err(FontRasterizerError::GlyphNotFound(c))
    }

    pub(crate) fn convert(
        &self,
        c: char,
        width: CharWidth,
    ) -> Result<GlyphVertex, FontRasterizerError> {
        let (
            face,
            CharGlyphIds {
                horizontal_glyph_id,
                vertical_glyph_id,
            },
        ) = self.get_face_and_glyph_ids(c)?;
        let h_vertex = GlyphVertexBuilder::new().build(horizontal_glyph_id, width, &face)?;
        let v_vertex = vertical_glyph_id.and_then(|vertical_glyph_id| {
            GlyphVertexBuilder::new()
                .build(vertical_glyph_id, width, &face)
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
        width: CharWidth,
        face: &Face,
    ) -> Result<VectorVertex, FontRasterizerError> {
        let mut builder = VectorVertexBuilder::new();

        let rect = face
            .outline_glyph(glyph_id, &mut builder)
            .ok_or(FontRasterizerError::NoOutlineGlyph(glyph_id))?;

        let global = face.global_bounding_box();
        let global_width = global.width() as f32;
        let global_height = global.height() as f32;
        let rect_em = face.units_per_em() as f32;
        let center_x = global_width * (width.to_f32() / 2.0) + global.x_min as f32;
        let center_y = global_height / 2.0 + global.y_min as f32;

        let mut builder =
            builder.with_options(VertexBuilderOptions::new([center_x, center_y], rect_em));

        if DEBUG_FLAGS.show_glyph_outline {
            // global
            builder.move_to(global.x_min as f32, global.y_min as f32);
            builder.line_to(global.x_max as f32, global.y_min as f32);
            builder.line_to(global.x_max as f32, global.y_max as f32);
            builder.line_to(global.x_min as f32, global.y_max as f32);
            builder.line_to(global.x_min as f32, global.y_min as f32);
            // rect
            builder.move_to(rect.x_min as f32, rect.y_min as f32);
            builder.line_to(rect.x_max as f32, rect.y_min as f32);
            builder.line_to(rect.x_max as f32, rect.y_max as f32);
            builder.line_to(rect.x_min as f32, rect.y_max as f32);
            builder.line_to(rect.x_min as f32, rect.y_min as f32);

            // center
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
    use rustybuzz::ttf_parser::Face;

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
        let converter = FontVertexConverter::new(Arc::new(font_binaries));

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
        let faces = [
            Face::parse(FONT_DATA, 0).expect("face from slice"),
            Face::parse(EMOJI_FONT_DATA, 0).expect("face from slice"),
        ];

        for face in faces.iter() {
            println!("-----------------");
            let global_box = face.global_bounding_box();
            println!(
                "global:{:?}, width:{}, height:{}",
                global_box,
                global_box.width(),
                global_box.height()
            );
            println!(
                "em:{:?}, origin_rect_em: {}, new_rect_em: {}",
                face.units_per_em(),
                (face.units_per_em() as f32 / 1024.0).sqrt(),
                (face.units_per_em() as f32).sqrt()
            );
        }
    }
}
