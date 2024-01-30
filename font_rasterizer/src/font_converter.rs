use std::sync::Arc;

use anyhow::Context;
use bezier_converter::CubicBezier;
use font_collector::FontData;
use log::debug;
use rustybuzz::ttf_parser::{Face, OutlineBuilder};
use unicode_width::UnicodeWidthChar;

use crate::debug_mode::DEBUG_FLAGS;

pub(crate) struct FontVertexConverter {
    fonts: Vec<FontData>,
}

impl FontVertexConverter {
    pub(crate) fn new(fonts: Vec<FontData>) -> Self {
        Self { fonts }
    }

    pub(crate) fn convert(&self, c: char) -> anyhow::Result<GlyphVertex> {
        let faces = self
            .fonts
            .iter()
            .flat_map(|f| Face::parse(&f.binary, f.index))
            .collect::<Vec<Face>>();
        for face in faces.iter() {
            if let Ok(glyph) = GlyphVertexBuilder::new().build(c, &face) {
                return Ok(glyph);
            }
        }
        anyhow::bail!("no glyph. char:{}", c)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GlyphWidth {
    Regular,
    Wide,
}

impl GlyphWidth {
    fn get_width(c: char, face: &Face) -> Self {
        if let Some(glyph_id) = face.glyph_index(c) {
            if let Some(rect) = face.glyph_bounding_box(glyph_id) {
                if face.global_bounding_box().width() < rect.width() * 2 {
                    return GlyphWidth::Wide;
                }
            }
        }
        match UnicodeWidthChar::width_cjk(c) {
            Some(1) => GlyphWidth::Regular,
            Some(_) => GlyphWidth::Wide,
            None => GlyphWidth::Regular,
        }
    }

    /// 描画時に左にどれぐらい移動させるか
    pub fn left(&self) -> f32 {
        match self {
            GlyphWidth::Regular => -0.25,
            GlyphWidth::Wide => 0.0,
        }
    }

    /// 描画時に右にどれぐらい移動させるか
    pub fn right(&self) -> f32 {
        match self {
            GlyphWidth::Regular => 0.75,
            GlyphWidth::Wide => 1.0,
        }
    }

    /// グリフ自体の横幅
    pub fn to_f32(self) -> f32 {
        match self {
            GlyphWidth::Regular => 0.5,
            GlyphWidth::Wide => 1.0,
        }
    }
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
    pub(crate) vertex: Vec<Vertex>,
    pub(crate) index: Vec<u32>,
    pub(crate) width: GlyphWidth,
}

impl GlyphVertex {
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

    pub(crate) fn build(mut self, c: char, face: &Face) -> anyhow::Result<GlyphVertex> {
        let glyph_id = face
            .glyph_index(c)
            .with_context(|| format!("no glyph. char:{}", c))?;
        let rect = face
            .outline_glyph(glyph_id, &mut self)
            .with_context(|| format!("ougline glyph is afiled. char:{}", c))?;

        let width = GlyphWidth::get_width(c, face);
        let global = face.global_bounding_box();
        let global_width = global.width() as f32;
        let global_height = global.height() as f32;
        let rect_em = (face.units_per_em() as f32 / 1024.0).sqrt();
        let center_x = global_width * width.to_f32() / 2.0 + global.x_min as f32;
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
                let x = (*x - center_x) / global_width / rect_em;
                let y = (*y - center_y) / global_height / rect_em;
                Vertex {
                    position: [x, y],
                    wait: wait.wait(),
                }
            })
            .collect();
        Ok(GlyphVertex {
            c,
            vertex,
            index: self.index,
            width,
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
