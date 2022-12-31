use std::{
    collections::{BTreeMap, HashSet},
    ops::{Range, RangeInclusive},
};

use anyhow::Context;
use log::{debug, info};
use ttf_parser::{Face, OutlineBuilder, Rect};
use wgpu::util::DeviceExt;

const FONT_DATA: &[u8] = include_bytes!("../../wgpu_gui/src/font/HackGenConsole-Regular.ttf");

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

impl FontVertex {
    pub(crate) fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<FontVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

struct FontVertexBuilder {
    main_vertex: Vec<InternalFontVertex>,
    main_index: Vec<u32>,
    current_main_index: u32,
    vertex_swap: bool,

    flushed_vertex: Vec<FontVertex>,
    flushed_index: Vec<u32>,

    index_range: BTreeMap<char, Range<u32>>,
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
        }
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

    fn flush(&mut self, c: char, rect: Rect) {
        let range = self.flushed_index.len() as u32
            ..(self.flushed_index.len() + self.main_index.len()) as u32;

        let mut vertex = self
            .main_vertex
            .iter()
            .map(|InternalFontVertex { x, y, wait }| {
                let x = (*x / rect.width() as f32) - 0.5;
                let y = (*y / rect.height() as f32) - 0.5;
                FontVertex {
                    position: [x, y],
                    wait: [wait.0, wait.1],
                }
            })
            .collect();
        self.flushed_vertex.append(&mut vertex);
        self.flushed_index.append(&mut self.main_index);
        self.index_range.insert(c, range);
        self.main_vertex.clear();
        self.main_index.clear();
    }

    fn build(self) -> (Vec<FontVertex>, Vec<u32>, BTreeMap<char, Range<u32>>) {
        info!(
            "vertex:{}, index:{}, polygon:{}, char:{}",
            self.flushed_vertex.len(),
            self.flushed_index.len(),
            self.flushed_index.len() / 3,
            self.index_range.len(),
        );
        (self.flushed_vertex, self.flushed_index, self.index_range)
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

    pub(crate) fn new_buffer(
        device: &wgpu::Device,
        chars: Vec<RangeInclusive<char>>,
    ) -> anyhow::Result<Self> {
        let face = Face::parse(FONT_DATA, 0)?;

        let chars: HashSet<char> = chars
            .into_iter()
            .flat_map(|char_range| char_range.collect::<Vec<_>>())
            .collect();

        let mut builder = FontVertexBuilder::new();
        for c in chars {
            let Some(glyph_id) = face.glyph_index(c) else {
                debug!("no glyph. char:{}", c);
                continue};
            let Some(rect) = face
                .outline_glyph(glyph_id, &mut builder)
                else {
                    debug!("ougline glyph is failed. char:{}", c);
                    continue};
            builder.flush(c, rect);
        }
        let (vertex_buffer, index_buffer, index_range) = builder.build();

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
        })
    }

    pub(crate) fn range(&self, c: char) -> anyhow::Result<Range<u32>> {
        self.index_range
            .get(&c)
            .map(|r| r.clone())
            .with_context(|| "get char")
    }
}
