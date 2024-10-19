use std::sync::Arc;

use anyhow::Context;
use bezier_converter::CubicBezier;
use font_collector::FontData;
use log::debug;
use rustybuzz::{
    shape,
    ttf_parser::{GlyphId, OutlineBuilder},
    Direction, Face, UnicodeBuffer,
};

use crate::{char_width_calcurator::CharWidth, debug_mode::DEBUG_FLAGS};

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

    fn get_face_and_glyph_ids(&self, c: char) -> anyhow::Result<(Face, CharGlyphIds)> {
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
        anyhow::bail!("no glyph. char:{}", c)
    }

    pub(crate) fn convert(&self, c: char, width: CharWidth) -> anyhow::Result<GlyphVertex> {
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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Vertex {
    pub(crate) position: [f32; 2],
    // ベジエ曲線を描くために 3 頂点のうちどれを制御点、どれを始点・終点と区別するかを表す。
    // 典型的には [0, 0], または [0, 1] が始点か終点。[1, 0] 制御点となる。
    pub(crate) wait: [f32; 2],
}

#[derive(Clone, Copy)]
enum FlipFlop {
    Flip,
    Flop,
    Control,
}

impl FlipFlop {
    fn next(&self) -> Self {
        match self {
            FlipFlop::Flip => FlipFlop::Flop,
            FlipFlop::Flop => FlipFlop::Flip,
            FlipFlop::Control => FlipFlop::Control,
        }
    }

    fn wait(&self) -> [f32; 2] {
        match self {
            FlipFlop::Flip => [0.0, 0.0],
            FlipFlop::Flop => [0.0, 1.0],
            FlipFlop::Control => [1.0, 0.0],
        }
    }
}

struct InternalVertex {
    x: f32,
    y: f32,
    wait: FlipFlop,
}

pub(crate) struct GlyphVertex {
    pub(crate) c: char,
    pub(crate) h_vertex: GlyphVertexData,
    pub(crate) v_vertex: Option<GlyphVertexData>,
}

pub(crate) struct GlyphVertexData {
    pub(crate) vertex: Vec<Vertex>,
    pub(crate) index: Vec<u32>,
}

impl GlyphVertexData {
    pub fn vertex_size(&self) -> u64 {
        (self.vertex.len() * std::mem::size_of::<Vertex>()) as u64
    }
    pub fn index_size(&self) -> u64 {
        (self.index.len() * std::mem::size_of::<u32>()) as u64
    }
}

pub struct GlyphVertexBuilder {
    vertex: Vec<InternalVertex>,
    index: Vec<u32>,
    current_index: u32,
    vertex_swap: FlipFlop,
}

impl GlyphVertexBuilder {
    pub(crate) fn new() -> Self {
        Self {
            vertex: Vec::new(),
            index: Vec::new(),
            current_index: 0,
            vertex_swap: FlipFlop::Flip,
        }
    }

    #[inline]
    fn next_wait(&mut self) -> FlipFlop {
        self.vertex_swap = self.vertex_swap.next();
        self.vertex_swap
    }

    pub(crate) fn build(
        mut self,
        glyph_id: GlyphId,
        width: CharWidth,
        face: &Face,
    ) -> anyhow::Result<GlyphVertexData> {
        let rect = face
            .outline_glyph(glyph_id, &mut self)
            .with_context(|| format!("ougline glyph is afiled. glyph_id:{:?}", glyph_id))?;

        let global = face.global_bounding_box();
        let global_width = global.width() as f32;
        let global_height = global.height() as f32;
        let rect_em = face.units_per_em() as f32;
        let center_x = global_width * (width.to_f32() / 2.0) + global.x_min as f32;
        let center_y = global_height / 2.0 + global.y_min as f32;

        if DEBUG_FLAGS.show_glyph_outline {
            // global
            self.move_to(global.x_min as f32, global.y_min as f32);
            self.line_to(global.x_max as f32, global.y_min as f32);
            self.line_to(global.x_max as f32, global.y_max as f32);
            self.line_to(global.x_min as f32, global.y_max as f32);
            self.line_to(global.x_min as f32, global.y_min as f32);
            // rect
            self.move_to(rect.x_min as f32, rect.y_min as f32);
            self.line_to(rect.x_max as f32, rect.y_min as f32);
            self.line_to(rect.x_max as f32, rect.y_max as f32);
            self.line_to(rect.x_min as f32, rect.y_max as f32);
            self.line_to(rect.x_min as f32, rect.y_min as f32);

            // center
            let x = center_x;
            let y = center_y;
            self.move_to(x - 100.0, y);
            self.line_to(x, y + 100.0);
            self.line_to(x + 100.0, y);
            self.line_to(x, y - 100.0);
            self.line_to(x - 100.0, y);
        }

        let vertex = self
            .vertex
            .iter()
            .map(|InternalVertex { x, y, wait }| {
                let x = (*x - center_x) / rect_em;
                let y = (*y - center_y) / rect_em;
                Vertex {
                    position: [x, y],
                    wait: wait.wait(),
                }
            })
            .collect();
        Ok(GlyphVertexData {
            vertex,
            index: self.index,
        })
    }
}

impl OutlineBuilder for GlyphVertexBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        let wait = self.next_wait();
        self.vertex.push(InternalVertex { x, y, wait });
        self.current_index += 1;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let wait = self.next_wait();
        self.vertex.push(InternalVertex { x, y, wait });
        self.index.push(0);
        self.index.push(self.current_index);
        self.index.push(self.current_index + 1);
        self.current_index += 1;
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let wait = self.next_wait();

        self.vertex.push(InternalVertex {
            x: x1,
            y: y1,
            wait: FlipFlop::Control,
        });
        self.vertex.push(InternalVertex { x, y, wait });

        self.index.push(0);
        self.index.push(self.current_index);
        self.index.push(self.current_index + 2);

        // ベジエ曲線
        self.index.push(self.current_index);
        self.index.push(self.current_index + 1);
        self.index.push(self.current_index + 2);
        self.current_index += 2;
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        // 3 次ベジエを 2 次ベジエに近似する
        let last = &self.vertex[(self.current_index - 1) as usize];
        let cb = CubicBezier {
            x0: last.x,
            y0: last.y,
            x1: x,
            y1: y,
            cx0: x1,
            cy0: y1,
            cx1: x2,
            cy1: y2,
        };
        let qbs = cb.to_quadratic();
        debug!("cubic to quadratic: 1 -> {}", qbs.len());
        for qb in qbs.iter() {
            self.quad_to(qb.cx0, qb.cy0, qb.x1, qb.y1)
        }
    }

    fn close(&mut self) {
        // noop
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use font_collector::FontCollector;
    use rustybuzz::ttf_parser::Face;

    use super::FontVertexConverter;

    const FONT_DATA: &[u8] = include_bytes!("../examples/font/BIZUDMincho-Regular.ttf");
    const EMOJI_FONT_DATA: &[u8] = include_bytes!("../examples/font/NotoEmoji-Regular.ttf");

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
            Face::parse(
                include_bytes!("../examples/font/NotoSansMonoCJKjp-Regular.otf"),
                0,
            )
            .expect("face from slice"),
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
