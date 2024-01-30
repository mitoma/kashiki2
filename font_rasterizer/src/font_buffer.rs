use std::{
    collections::{BTreeMap, HashSet},
    ops::Range,
};

use anyhow::Context;

use font_collector::FontData;
use log::{debug, info};
use wgpu::BufferUsages;

use crate::font_converter::{FontVertexConverter, GlyphWidth, Vertex};

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

pub struct GlyphVertexBuffer {
    font_vertex_converter: FontVertexConverter,
    buffer_index: BTreeMap<char, BufferIndexEntry>,
    vertex_buffers: Vec<VertexBuffer>,
    index_buffers: Vec<IndexBuffer>,
}

impl GlyphVertexBuffer {
    pub fn new(font_binaries: Vec<FontData>) -> GlyphVertexBuffer {
        let font_vertex_converter = FontVertexConverter::new(font_binaries);
        GlyphVertexBuffer {
            font_vertex_converter,
            buffer_index: BTreeMap::default(),
            vertex_buffers: Vec::new(),
            index_buffers: Vec::new(),
        }
    }

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

    pub fn append_glyph(
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

        // char を全て Glyph 情報に変換する
        let mut glyphs = chars
            .iter()
            .flat_map(|c| self.font_vertex_converter.convert(*c))
            .collect::<Vec<_>>();

        // 空いている vertex, index バッファを探し、無ければバッファを作る
        // バッファが見つかったらその buffer に書き込むキューを登録する
        while let Some(glyph) = glyphs.pop() {
            let Some(vertex_buffer_index) =
                self.appendable_vertex_buffer_index(glyph.vertex_size())
            else {
                self.vertex_buffers.push(VertexBuffer::new(
                    device,
                    queue,
                    format!("glyph vertex buffer #{}", self.index_buffers.len()),
                ));
                glyphs.push(glyph);
                continue;
            };
            let Some(index_buffer_index) = self.appendable_index_buffer_index(glyph.index_size())
            else {
                self.index_buffers.push(IndexBuffer::new(
                    device,
                    format!("glyph index buffer #{}", self.index_buffers.len()),
                ));
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

    pub fn width(&self, c: char) -> GlyphWidth {
        let draw_info = self.draw_info(&c);
        draw_info
            .map(|i| i.glyph_width)
            .cloned()
            .unwrap_or(GlyphWidth::Regular)
    }
}
