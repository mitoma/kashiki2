use std::{
    collections::{BTreeMap, HashSet},
    ops::{Range, RangeInclusive},
};

use anyhow::Context;
use log::{debug, info};
use ttf_parser::{Face, GlyphId, OutlineBuilder, Rect};
use unicode_width::UnicodeWidthChar;
use wgpu::util::DeviceExt;

const FONT_DATA: &[u8] = include_bytes!("../../wgpu_gui/src/font/HackGenConsole-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] = include_bytes!("../../wgpu_gui/src/font/NotoEmoji-Regular.ttf");

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum GlyphWidth {
    Regular,
    Wide,
}

impl GlyphWidth {
    fn get_width(c: char, glyph_id: GlyphId, face: &Face) -> Self {
        if let Some(rect) = face.glyph_bounding_box(glyph_id) {
            if face.global_bounding_box().width() < rect.width() * 2 {
                return GlyphWidth::Wide;
            }
        }
        match UnicodeWidthChar::width_cjk(c) {
            Some(1) => GlyphWidth::Regular,
            Some(_) => GlyphWidth::Wide,
            None => GlyphWidth::Regular,
        }
    }

    pub(crate) fn left(&self) -> f32 {
        match self {
            GlyphWidth::Regular => -0.25,
            GlyphWidth::Wide => 0.0,
        }
    }
    pub(crate) fn right(&self) -> f32 {
        match self {
            GlyphWidth::Regular => 0.75,
            GlyphWidth::Wide => 1.0,
        }
    }

    pub(crate) fn to_f32(&self) -> f32 {
        match self {
            GlyphWidth::Regular => 0.5,
            GlyphWidth::Wide => 1.0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct FontVertex {
    pub(crate) position: [f32; 2],
    pub(crate) wait: [f32; 2],
}

struct InternalFontVertex {
    x: f32,
    y: f32,
    wait: (f32, f32),
}

struct FontVertexBuilder {
    main_vertex: Vec<InternalFontVertex>,
    main_index: Vec<u32>,
    current_main_index: u32,
    vertex_swap: bool,

    flushed_vertex: Vec<FontVertex>,
    flushed_index: Vec<u32>,

    index_range: BTreeMap<char, Range<u32>>,
    glyph_width: BTreeMap<char, GlyphWidth>,
}

impl FontVertexBuilder {
    fn new() -> Self {
        FontVertexBuilder {
            main_vertex: Vec::new(),
            main_index: Vec::new(),
            current_main_index: 0,
            vertex_swap: false,
            flushed_vertex: vec![FontVertex {
                position: [0.0; 2],
                wait: [0.0; 2],
            }],
            flushed_index: Vec::new(),
            index_range: BTreeMap::new(),
            glyph_width: BTreeMap::new(),
        }
    }

    #[allow(dead_code)]
    fn add_debug_mark(&mut self) {
        let (w1, w2) = self.next_wait();
        let wait1 = [w1, w2];
        let (w1, w2) = self.next_wait();
        let wait2 = [w1, w2];
        let (w1, w2) = self.next_wait();
        let wait3 = [w1, w2];
        let (w1, w2) = self.next_wait();
        let wait4 = [w1, w2];
        self.flushed_vertex.append(&mut vec![
            FontVertex {
                position: [-0.1, -0.1],
                wait: wait1,
            },
            FontVertex {
                position: [-0.1, 0.1],
                wait: wait2,
            },
            FontVertex {
                position: [0.1, -0.1],
                wait: wait3,
            },
            FontVertex {
                position: [0.1, 0.1],
                wait: wait4,
            },
        ]);
        self.flushed_index.append(&mut vec![
            0,
            self.current_main_index + 1,
            self.current_main_index + 2,
            0,
            self.current_main_index + 3,
            self.current_main_index + 4,
        ]);
        self.current_main_index += 4;
    }

    #[inline]
    fn next_wait(&mut self) -> (f32, f32) {
        self.vertex_swap = !self.vertex_swap;
        if self.vertex_swap {
            (0.0, 1.0)
        } else {
            (0.0, 0.0)
        }
    }

    fn flush(&mut self, c: char, face: &Face, glyph_id: GlyphId, rect: Rect) {
        let range = self.flushed_index.len() as u32
            ..(self.flushed_index.len() + self.main_index.len()) as u32;
        let glyph_width = GlyphWidth::get_width(c, glyph_id, &face);
        info!(
            r#"
            char: {}
                global: [rect:{:?}, width:{}, height:{}]
                glyph : [rect:{:?}, width:{}, height:{}]
                em:{}, glyph_width:{:?}
            "#,
            c,
            face.global_bounding_box(),
            face.global_bounding_box().width(),
            face.global_bounding_box().height(),
            rect,
            rect.width(),
            rect.height(),
            face.units_per_em(),
            glyph_width,
        );
        let global = face.global_bounding_box();
        let rect_width = rect.width() as f32;
        let rect_xmin = rect.x_min as f32;
        let global_width = global.width() as f32;
        let global_height = global.height() as f32;
        let capital_height = face.capital_height().unwrap() as f32;
        let rect_em = (face.units_per_em() as f32 / 1024.0).sqrt();

        let mut vertex = self
            .main_vertex
            .iter()
            .map(|InternalFontVertex { x, y, wait }| {
                let x = (*x - rect_xmin - rect_width / 2.0) / global_width / rect_em;
                let y = (*y - capital_height / 2.0) / global_height / rect_em;
                FontVertex {
                    position: [x, y],
                    wait: [wait.0, wait.1],
                }
            })
            .collect();

        self.flushed_vertex.append(&mut vertex);
        self.flushed_index.append(&mut self.main_index);

        //self.add_debug_mark();
        //self.index_range.insert(c, range.start..range.end + 6);

        self.index_range.insert(c, range.start..range.end);
        self.glyph_width.insert(c, glyph_width);
        self.main_vertex.clear();
        self.main_index.clear();
    }

    fn build(
        self,
    ) -> (
        Vec<FontVertex>,
        Vec<u32>,
        BTreeMap<char, Range<u32>>,
        BTreeMap<char, GlyphWidth>,
    ) {
        info!(
            "vertex:{}, index:{}, polygon:{}, char:{}",
            self.flushed_vertex.len(),
            self.flushed_index.len(),
            self.flushed_index.len() / 3,
            self.index_range.len(),
        );
        (
            self.flushed_vertex,
            self.flushed_index,
            self.index_range,
            self.glyph_width,
        )
    }
}

impl OutlineBuilder for FontVertexBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        let wait = self.next_wait();
        self.main_vertex.push(InternalFontVertex { x, y, wait });
        self.current_main_index += 1;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let wait = self.next_wait();
        self.main_vertex.push(InternalFontVertex { x, y, wait });
        self.main_index.push(0);
        self.main_index.push(self.current_main_index);
        self.main_index.push(self.current_main_index + 1);
        self.current_main_index += 1;
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let wait = self.next_wait();

        self.main_vertex.push(InternalFontVertex {
            x: x1,
            y: y1,
            wait: (1.0, 0.0),
        });
        self.main_vertex.push(InternalFontVertex { x, y, wait });

        self.main_index.push(0);
        self.main_index.push(self.current_main_index);
        self.main_index.push(self.current_main_index + 2);

        // ベジエ曲線
        self.main_index.push(self.current_main_index);
        self.main_index.push(self.current_main_index + 1);
        self.main_index.push(self.current_main_index + 2);
        self.current_main_index += 2;
    }

    fn curve_to(&mut self, _x1: f32, _y1: f32, _x2: f32, _y2: f32, _x: f32, _y: f32) {
        todo!("実装予定無し！(OpenTypeをサポートしたいときには必要かもね)")
    }

    fn close(&mut self) {
        // noop
    }
}

pub(crate) struct FontVertexBuffer {
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    index_range: BTreeMap<char, Range<u32>>,
    glyph_width: BTreeMap<char, GlyphWidth>,
}

impl FontVertexBuffer {
    pub(crate) fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<FontVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // 文字情報なので xy の座標だけでよい
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // ベジエか直線かの情報が必要なので [f32; 2] を使っている。
                // 本質的には 2 bit でいいはずなので調整余地あり
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }

    fn build(
        builder: &mut FontVertexBuilder,
        c: char,
        f: &Face,
    ) -> anyhow::Result<(GlyphId, Rect)> {
        let glyph_id = f
            .glyph_index(c)
            .with_context(|| format!("no glyph. char:{}", c))?;
        f.outline_glyph(glyph_id, builder)
            .map(|rect| (glyph_id, rect))
            .with_context(|| format!("ougline glyph is failed. char:{}", c))
    }

    pub(crate) fn new_buffer(
        device: &wgpu::Device,
        chars: Vec<RangeInclusive<char>>,
    ) -> anyhow::Result<Self> {
        let face = Face::parse(FONT_DATA, 0)?;
        let emoji_face = Face::parse(EMOJI_FONT_DATA, 0)?;
        let faces = vec![face, emoji_face];

        let chars: HashSet<char> = chars
            .into_iter()
            .flat_map(|char_range| char_range.collect::<Vec<_>>())
            .collect();

        let mut builder = FontVertexBuilder::new();
        for c in chars {
            let Some((face,glyph_id, rect)) = faces
                .iter()
                .flat_map(|face| Self::build(&mut builder, c, face).map(|(glyph_id, rect)|(face, glyph_id, rect)))
                .next() else {
                    debug!("font にない文字です。 char:{}", c);
                    continue};
            builder.flush(c, face, glyph_id, rect);
        }
        let (vertex_buffer, index_buffer, index_range, glyph_width) = builder.build();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Overlap Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertex_buffer),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Overlap Index Buffer"),
            contents: bytemuck::cast_slice(&index_buffer),
            usage: wgpu::BufferUsages::INDEX,
        });

        Ok(Self {
            vertex_buffer,
            index_buffer,
            index_range,
            glyph_width,
        })
    }

    pub(crate) fn range(&self, c: char) -> anyhow::Result<Range<u32>> {
        self.index_range
            .get(&c)
            .map(|r| r.clone())
            .with_context(|| "get char")
    }

    pub(crate) fn width(&self, c: char) -> GlyphWidth {
        self.glyph_width
            .get(&c)
            .map(|r| *r)
            .unwrap_or(GlyphWidth::Regular)
    }
}
