use std::{
    collections::{BTreeMap, HashSet},
    ops::Range,
};

use anyhow::Context;

use log::{debug, info};
use ttf_parser::{Face, OutlineBuilder};
use unicode_width::UnicodeWidthChar;
use wgpu::BufferUsages;

const FONT_DATA: &[u8] = include_bytes!("../../wgpu_gui/src/font/HackGenConsole-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] = include_bytes!("../../wgpu_gui/src/font/NotoEmoji-Regular.ttf");

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum GlyphWidth {
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

    pub(crate) fn to_f32(self) -> f32 {
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

struct GlyphVertex {
    c: char,
    vertex: Vec<Vertex>,
    index: Vec<u32>,
    width: GlyphWidth,
}
impl GlyphVertex {
    fn vertex_size(&self) -> u64 {
        (self.vertex.len() * std::mem::size_of::<Vertex>()) as u64
    }
    fn index_size(&self) -> u64 {
        (self.index.len() * std::mem::size_of::<u32>()) as u64
    }
}

struct GlyphVertexBuilder {
    vertex: Vec<InternalVertex>,
    index: Vec<u32>,
    current_index: u32,
    vertex_swap: FlipFlop,
}

impl GlyphVertexBuilder {
    fn new() -> Self {
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

    fn build(mut self, c: char, face: &Face) -> anyhow::Result<GlyphVertex> {
        let glyph_id = face
            .glyph_index(c)
            .with_context(|| format!("no glyph. char:{}", c))?;
        let rect = face
            .outline_glyph(glyph_id, &mut self)
            .with_context(|| format!("ougline glyph is afiled. char:{}", c))?;

        let width = GlyphWidth::get_width(c, face);
        let global = face.global_bounding_box();
        let rect_width = rect.width() as f32;
        let rect_xmin = rect.x_min as f32;
        let global_width = global.width() as f32;
        let global_height = global.height() as f32;
        let capital_height = face.capital_height().unwrap() as f32;
        let rect_em = (face.units_per_em() as f32 / 1024.0).sqrt();

        let vertex = self
            .vertex
            .iter()
            .map(|InternalVertex { x, y, wait }| {
                let x = (*x - rect_xmin - rect_width / 2.0) / global_width / rect_em;
                let y = (*y - capital_height / 2.0) / global_height / rect_em;
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

    fn curve_to(&mut self, _x1: f32, _y1: f32, _x2: f32, _y2: f32, _x: f32, _y: f32) {
        todo!("実装予定無し！(OpenTypeをサポートしたいときには必要かもね)")
    }

    fn close(&mut self) {
        // noop
    }
}

#[derive(Default)]
pub(crate) struct GlyphVertexBuffer {
    buffer_index: BTreeMap<char, BufferIndexEntry>,
    vertex_buffers: Vec<VertexBuffer>,
    index_buffers: Vec<IndexBuffer>,
}

struct BufferIndexEntry {
    vertex_buffer_index: usize,
    index_buffer_index: usize,
    index_buffer_range: Range<u32>,
    glyph_width: GlyphWidth,
}

// バッファを 1M ずつ確保する
const BUFFER_SIZE: u64 = 1_048_576;

const ZERO_VERTEX: Vertex = Vertex {
    position: [0.0, 0.0],
    wait: [0.0, 0.0],
};

struct VertexBuffer {
    wgpu_buffer: wgpu::Buffer,
    offset: u64,
}

impl VertexBuffer {
    fn capacity(&self) -> u64 {
        BUFFER_SIZE - self.offset
    }

    fn new(device: &wgpu::Device, queue: &wgpu::Queue, label: String) -> Self {
        let wgpu_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&label),
            size: BUFFER_SIZE,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&wgpu_buffer, 0, bytemuck::cast_slice(&[ZERO_VERTEX]));
        Self {
            offset: std::mem::size_of::<Vertex>() as u64,
            wgpu_buffer,
        }
    }

    fn next_index_position(&self) -> u64 {
        // buffer の最初には常に原点の座標が入っているので index その分ずらす必要がある
        self.offset / std::mem::size_of::<Vertex>() as u64 - 1
    }
}

struct IndexBuffer {
    wgpu_buffer: wgpu::Buffer,
    offset: u64,
}

impl IndexBuffer {
    fn capacity(&self) -> u64 {
        BUFFER_SIZE - self.offset
    }

    fn new(device: &wgpu::Device, label: String) -> Self {
        let wgpu_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&label),
            size: BUFFER_SIZE,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            offset: 0,
            wgpu_buffer,
        }
    }

    fn next_range_position(&self) -> u32 {
        (self.offset / std::mem::size_of::<u32>() as u64) as u32
    }
}

#[derive(Debug)]
pub(crate) struct DrawInfo<'a> {
    pub(crate) vertex: &'a wgpu::Buffer,
    pub(crate) index: &'a wgpu::Buffer,
    pub(crate) index_range: &'a Range<u32>,
    pub(crate) glyph_width: &'a GlyphWidth,
}

impl GlyphVertexBuffer {
    pub(crate) fn draw_info(&self, c: &char) -> anyhow::Result<DrawInfo> {
        let index = &self
            .buffer_index
            .get(c)
            .with_context(|| format!("get char from buffer index. c:{}", c))?;
        let vertex_buffer = &self.vertex_buffers[index.vertex_buffer_index];
        let index_buffer = &self.index_buffers[index.index_buffer_index];
        let draw_info = DrawInfo {
            vertex: &vertex_buffer.wgpu_buffer,
            index: &index_buffer.wgpu_buffer,
            index_range: &index.index_buffer_range,
            glyph_width: &index.glyph_width,
        };
        Ok(draw_info)
    }

    pub(crate) fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
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

    pub(crate) fn append_glyph(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        chars: HashSet<char>,
    ) -> anyhow::Result<()> {
        // 既にバッファに登録済みの char は除外する。
        let chars = chars
            .into_iter()
            .filter(|c| !self.buffer_index.contains_key(c))
            .collect::<HashSet<_>>();
        if chars.is_empty() {
            return Ok(());
        }

        let face = Face::parse(FONT_DATA, 0)?;
        let emoji_face = Face::parse(EMOJI_FONT_DATA, 0)?;
        let faces = vec![face, emoji_face];

        // char を全て Glyph 情報に変換する
        let mut glyphs = chars
            .iter()
            .flat_map(|c| {
                faces
                    .iter()
                    .find_map(|face| GlyphVertexBuilder::new().build(*c, face).ok())
            })
            .collect::<Vec<_>>();

        // 空いている vertex, index バッファを探し、無ければバッファを作る
        // バッファが見つかったらその buffer に書き込むキューを登録する
        while let Some(glyph) = glyphs.pop() {
            let Some(vertex_buffer_index) = self.appendable_vertex_buffer_index(glyph.vertex_size()) else {
                self.vertex_buffers.push(VertexBuffer::new(device, queue, format!("glyph vertex buffer #{}", self.index_buffers.len())));
                glyphs.push(glyph);
                continue;
            };
            let Some(index_buffer_index) = self.appendable_index_buffer_index(glyph.index_size()) else {
                self.index_buffers.push(IndexBuffer::new(device,format!("glyph index buffer #{}", self.index_buffers.len())));
                glyphs.push(glyph);
                continue;
            };

            debug!(
                "glyph vertex_size:{}, index_size:{}",
                glyph.vertex_size(),
                glyph.index_size()
            );

            let vertex_buffer = self.vertex_buffers.get_mut(vertex_buffer_index).unwrap();
            let next_index_position = vertex_buffer.next_index_position();
            debug!("pre vertex offset:{}", vertex_buffer.offset);
            queue.write_buffer(
                &vertex_buffer.wgpu_buffer,
                vertex_buffer.offset,
                bytemuck::cast_slice(&glyph.vertex),
            );
            vertex_buffer.offset += glyph.vertex_size();
            debug!("post vertex offset:{}", vertex_buffer.offset);
            debug!("next_index_position :{}", next_index_position);

            let index_buffer = self.index_buffers.get_mut(index_buffer_index).unwrap();
            let range_start = index_buffer.next_range_position();
            // vertex buffer に既に入っている座標の分だけ index をずらす
            let data = glyph
                .index
                .iter()
                .map(|idx| {
                    if *idx != 0 {
                        idx + next_index_position as u32
                    } else {
                        0
                    }
                })
                .collect::<Vec<u32>>();
            queue.write_buffer(
                &index_buffer.wgpu_buffer,
                index_buffer.offset,
                bytemuck::cast_slice(&data),
            );
            index_buffer.offset += glyph.index_size();
            let range_end = index_buffer.next_range_position();

            debug!(
                "char:{},  vertex_len:{}, vertex:{:?}, data_len: {}, data: {:?}, range:{}..{}",
                glyph.c,
                glyph.vertex.len(),
                glyph.vertex,
                data.len(),
                data,
                range_start,
                range_end
            );

            self.buffer_index.insert(
                glyph.c,
                BufferIndexEntry {
                    vertex_buffer_index,
                    index_buffer_index,
                    index_buffer_range: range_start..range_end,
                    glyph_width: glyph.width,
                },
            );
        }

        info!(
            "chars:{}, vertex_buffers:{}, index_buffers:{}",
            self.buffer_index.len(),
            self.vertex_buffers.len(),
            self.index_buffers.len()
        );

        Ok(())
    }

    fn appendable_vertex_buffer_index(&self, size: u64) -> Option<usize> {
        self.vertex_buffers
            .iter()
            .enumerate()
            .find(|(_, b)| b.capacity() >= size)
            .map(|r| r.0)
    }

    fn appendable_index_buffer_index(&self, size: u64) -> Option<usize> {
        self.index_buffers
            .iter()
            .enumerate()
            .find(|(_, b)| b.capacity() >= size)
            .map(|r| r.0)
    }

    pub(crate) fn width(&self, c: char) -> GlyphWidth {
        let draw_info = self.draw_info(&c);
        draw_info
            .map(|i| i.glyph_width)
            .cloned()
            .unwrap_or(GlyphWidth::Regular)
    }
}