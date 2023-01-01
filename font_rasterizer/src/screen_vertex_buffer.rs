use std::ops::Range;
use wgpu::util::DeviceExt;

/// スクリーン全体を覆うテクスチャを表示するための座標情報を扱うバッファ
pub(crate) struct ScreenVertexBuffer {
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    pub(crate) index_range: Range<u32>,
}

impl ScreenVertexBuffer {
    pub(crate) fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ScreenVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }

    pub(crate) fn new_buffer(device: &wgpu::Device) -> anyhow::Result<Self> {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Screen Vertex Buffer"),
            contents: bytemuck::cast_slice(SCREEN_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Screen Index Buffer"),
            contents: bytemuck::cast_slice(SCREEN_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        Ok(Self {
            vertex_buffer,
            index_buffer,
            index_range: 0..SCREEN_INDICES.len() as u32,
        })
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ScreenVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

const SCREEN_VERTICES: &[ScreenVertex] = &[
    ScreenVertex {
        position: [-1.0, -1.0, 0.0],
        tex_coords: [0.0, 1.0],
    },
    ScreenVertex {
        position: [1.0, -1.0, 0.0],
        tex_coords: [1.0, 1.0],
    },
    ScreenVertex {
        position: [-1.0, 1.0, 0.0],
        tex_coords: [0.0, 0.0],
    },
    ScreenVertex {
        position: [1.0, 1.0, 0.0],
        tex_coords: [1.0, 0.0],
    },
];

const SCREEN_INDICES: &[u16] = &[0, 1, 2, 2, 1, 3];
