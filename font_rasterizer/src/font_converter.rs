use std::sync::Arc;

use font_collector::FontData;
use log::info;
use rustybuzz::{
    Direction, Face, UnicodeBuffer, shape,
    ttf_parser::{GlyphId, Tag},
};
use ttf_overlap_remover::OverlapRemoveOutlineBuilder;

use crate::{
    char_width_calcurator::CharWidth,
    debug_mode::DEBUG_FLAGS,
    errors::FontRasterizerError,
    vector_vertex::{CoordinateSystem, VectorVertex, VectorVertexBuilder, VertexBuilderOptions},
    vector_vertex_v0::VectorVertexBuilderV0,
};

pub(crate) struct FontVertexConverter {
    fonts: Arc<Vec<FontData>>,
}

impl FontVertexConverter {
    pub(crate) fn new(fonts: Arc<Vec<FontData>>) -> Self {
        Self { fonts }
    }

    fn is_remove_outline_fontname(fontname: &str) -> bool {
        // Noto 系の文字を全部オーバーラップ除去対象にしてみる
        // TODO おそらくひたすら遅くなるはずなのでオーバーラップ除去処理結果をキャッシュする実装を追加したい
        //["Noto Emoji Regular"].contains(&fontname)
        fontname.contains("Noto")
    }

    fn faces(&'_ self) -> Vec<(Face<'_>, bool)> {
        self.fonts
            .iter()
            .flat_map(|f| {
                Face::from_slice(&f.binary, f.index).map(|face| {
                    // variable font の際に wght を Noto 系の Regular で指定されがちな 400 に指定する
                    // なぜなら、デフォルトだと 100 になってしまっておりやたら細くなってしまうからだ
                    // TODO 固定の指定ではなくて柔軟に、wght 以外のタグに指定できるようにしていく必要がある
                    let mut face = face.clone();
                    if face.is_variable() {
                        for axis in face.variation_axes() {
                            info!("variation: {}={}", axis.tag, axis.def_value);
                        }
                        for cood in face.variation_coordinates() {
                            info!("coordinate: {}", cood.get());
                        }

                        info!("set weight");
                        let _ = face.set_variation(Tag::from_bytes(b"wght"), 400.0);
                    } else {
                        info!("not variable");
                    }

                    (face, Self::is_remove_outline_fontname(&f.font_name))
                })
            })
            .collect::<Vec<_>>()
    }

    fn get_face_and_glyph_ids(
        &'_ self,
        c: char,
    ) -> Result<(Face<'_>, bool, CharGlyphIds), FontRasterizerError> {
        let mut buf = UnicodeBuffer::new();
        buf.set_direction(Direction::TopToBottom);
        buf.add(c, 0);
        for (face, remove_overlap) in self.faces().into_iter() {
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
                    remove_overlap,
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
            remove_overlap,
            CharGlyphIds {
                horizontal_glyph_id,
                vertical_glyph_id,
            },
        ) = self.get_face_and_glyph_ids(c)?;
        let h_vertex =
            GlyphVertexBuilder::new().build(horizontal_glyph_id, width, &face, remove_overlap)?;
        let v_vertex = vertical_glyph_id.and_then(|vertical_glyph_id| {
            GlyphVertexBuilder::new()
                .build(vertical_glyph_id, width, &face, remove_overlap)
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
        _width: CharWidth,
        face: &Face,
        remove_overlap: bool,
    ) -> Result<VectorVertex, FontRasterizerError> {
        let builder = VectorVertexBuilderV0::new();

        let rect_em = face.units_per_em() as f32;
        let center_x = face.glyph_hor_advance(glyph_id).unwrap() as f32 / 2.0;
        let center_y = face.capital_height().unwrap() as f32 / 2.0;

        let mut builder = builder.with_options(VertexBuilderOptions::new(
            [center_x, center_y],
            rect_em,
            CoordinateSystem::Font,
            None,
        ));

        let rect = if remove_overlap {
            let mut overlap_builder = OverlapRemoveOutlineBuilder::default();
            let rect = face
                .outline_glyph(glyph_id, &mut overlap_builder)
                .ok_or(FontRasterizerError::NoOutlineGlyph(glyph_id))?;
            overlap_builder.outline(&mut builder);
            rect
        } else {
            face.outline_glyph(glyph_id, &mut builder)
                .ok_or(FontRasterizerError::NoOutlineGlyph(glyph_id))?
        };

        if DEBUG_FLAGS.show_glyph_outline {
            let global = face.global_bounding_box();

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
            let (_, _, ids) = converter
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
