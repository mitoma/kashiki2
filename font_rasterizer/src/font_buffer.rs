use std::{
    collections::{BTreeMap, HashSet},
    ops::Range,
    sync::Arc,
};

use font_collector::FontData;
use log::{debug, info};
use text_buffer::editor::CharWidthResolver;
use wgpu::BufferUsages;

use crate::{
    char_width_calcurator::{CharWidth, CharWidthCalculator},
    errors::{BufferKind, FontRasterizerError},
    font_converter::{FontVertexConverter, GlyphVertex, GlyphVertexData, Vertex},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Direction {
    #[default]
    Horizontal,
    Vertical,
}

impl Direction {
    pub fn toggle(&self) -> Direction {
        match self {
            Direction::Horizontal => Direction::Vertical,
            Direction::Vertical => Direction::Horizontal,
        }
    }
}

// バッファに登録された文字のインデックス情報
struct BufferIndex {
    vertex_buffer_index: usize,
    index_buffer_index: usize,
    index_buffer_range: Range<u32>,
}

// 横書きと縦書きで別のバッファに登録されることがあるので、その情報を持つ
struct OrientationBufferIndices {
    horizontal: BufferIndex,
    vertical: Option<BufferIndex>,
}

impl OrientationBufferIndices {
    fn get_entry(&self, direction: &Direction) -> &BufferIndex {
        match direction {
            Direction::Horizontal => &self.horizontal,
            Direction::Vertical => self.vertical.as_ref().unwrap_or(&self.horizontal),
        }
    }
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
        // バッファの最初には常に原点の座標を入れておく
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
}

pub struct GlyphVertexBuffer {
    font_vertex_converter: FontVertexConverter,
    char_width_calculator: Arc<CharWidthCalculator>,
    buffer_index: BTreeMap<char, OrientationBufferIndices>,
    vertex_buffers: Vec<VertexBuffer>,
    index_buffers: Vec<IndexBuffer>,
}

impl GlyphVertexBuffer {
    pub fn new(
        font_binaries: Arc<Vec<FontData>>,
        char_width_calculator: Arc<CharWidthCalculator>,
    ) -> GlyphVertexBuffer {
        let font_vertex_converter = FontVertexConverter::new(font_binaries);
        GlyphVertexBuffer {
            font_vertex_converter,
            char_width_calculator,
            buffer_index: BTreeMap::default(),
            vertex_buffers: Vec::new(),
            index_buffers: Vec::new(),
        }
    }

    pub(crate) fn draw_info(
        &self,
        c: &char,
        direction: &Direction,
    ) -> Result<DrawInfo, FontRasterizerError> {
        let index = &self
            .buffer_index
            .get(c)
            .ok_or(FontRasterizerError::GlyphIndexNotFound)?
            .get_entry(direction);

        let vertex_buffer = &self.vertex_buffers[index.vertex_buffer_index];
        let index_buffer = &self.index_buffers[index.index_buffer_index];
        let draw_info = DrawInfo {
            vertex: &vertex_buffer.wgpu_buffer,
            index: &index_buffer.wgpu_buffer,
            index_range: &index.index_buffer_range,
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

    pub fn registerd_chars(&self) -> HashSet<char> {
        self.buffer_index.keys().cloned().collect()
    }

    pub fn append_glyph(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        chars: HashSet<char>,
    ) -> Result<(), FontRasterizerError> {
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
            .flat_map(|c| {
                let width = self.char_width_calculator.get_width(*c);
                self.font_vertex_converter.convert(*c, width)
            })
            .collect::<Vec<_>>();

        while let Some(glyph) = glyphs.pop() {
            let GlyphVertex {
                c,
                h_vertex,
                v_vertex,
            } = glyph;

            let h_entry = self.inner_append_glyph(device, queue, c, h_vertex)?;
            let v_entry = v_vertex
                .and_then(|v_vertex| self.inner_append_glyph(device, queue, c, v_vertex).ok());
            self.buffer_index.insert(
                c,
                OrientationBufferIndices {
                    horizontal: h_entry,
                    vertical: v_entry,
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

    fn inner_append_glyph(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        c: char,
        glyph_data: GlyphVertexData,
    ) -> Result<BufferIndex, FontRasterizerError> {
        self.ensure_buffer_capacity(device, queue, &glyph_data);

        let vertex_buffer_index = self
            .appendable_vertex_buffer_index(glyph_data.vertex_size())
            .ok_or(FontRasterizerError::BufferAllocationFailed(
                BufferKind::Vertex,
            ))?;

        let index_buffer_index = self
            .appendable_index_buffer_index(glyph_data.index_size())
            .ok_or(FontRasterizerError::BufferAllocationFailed(
                BufferKind::Index,
            ))?;

        // buffer に書き込むキューを登録する
        let vertex_buffer = self.vertex_buffers.get_mut(vertex_buffer_index).unwrap();
        let next_index_position = vertex_buffer.next_index_position();
        debug!("pre vertex offset:{}", vertex_buffer.offset);
        queue.write_buffer(
            &vertex_buffer.wgpu_buffer,
            vertex_buffer.offset,
            bytemuck::cast_slice(&glyph_data.vertex),
        );
        vertex_buffer.offset += glyph_data.vertex_size();
        debug!("post vertex offset:{}", vertex_buffer.offset);
        debug!("next_index_position :{}", next_index_position);

        let index_buffer = self.index_buffers.get_mut(index_buffer_index).unwrap();
        let range_start = index_buffer.next_range_position();
        // vertex buffer に既に入っている座標の分だけ index をずらす
        let data = glyph_data
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
        index_buffer.offset += glyph_data.index_size();
        let range_end = index_buffer.next_range_position();

        debug!(
            "char:{},  vertex_len:{}, vertex:{:?}, data_len: {}, data: {:?}, range:{}..{}",
            c,
            glyph_data.vertex.len(),
            glyph_data.vertex,
            data.len(),
            data,
            range_start,
            range_end
        );

        Ok(BufferIndex {
            vertex_buffer_index,
            index_buffer_index,
            index_buffer_range: range_start..range_end,
        })
    }

    // 空いている vertex, index バッファを探し、無ければバッファを作る
    fn ensure_buffer_capacity(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        glyph: &GlyphVertexData,
    ) {
        let vertex_size = glyph.vertex_size();
        let index_size = glyph.index_size();

        if self.appendable_vertex_buffer_index(vertex_size).is_none() {
            self.vertex_buffers.push(VertexBuffer::new(
                device,
                queue,
                format!("glyph vertex buffer #{}", self.vertex_buffers.len()),
            ));
        }

        if self.appendable_index_buffer_index(index_size).is_none() {
            self.index_buffers.push(IndexBuffer::new(
                device,
                format!("glyph index buffer #{}", self.index_buffers.len()),
            ));
        }
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

    pub fn width(&self, c: char) -> CharWidth {
        self.char_width_calculator.get_width(c)
    }
}

impl CharWidthResolver for GlyphVertexBuffer {
    fn resolve_width(&self, c: char) -> usize {
        match self.width(c) {
            CharWidth::Regular => 1,
            CharWidth::Wide => 2,
        }
    }
}
