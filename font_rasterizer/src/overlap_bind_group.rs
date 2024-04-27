use cgmath::SquareMatrix;
use wgpu::util::DeviceExt;

use crate::{screen_texture::ScreenTexture, time::now_millis};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    view_proj: [[f32; 4]; 4],
    default_view_proj: [[f32; 4]; 4],
    time: u32,
    // padding が必要らしい。正直意味わかんねぇな。
    padding: [u32; 3],
}

/// オーバーラップ用の BindGroup。
/// Uniforms として現在時刻のみ渡している。
pub struct OverlapBindGroup {
    uniforms: Uniforms,
    buffer: wgpu::Buffer,
    pub(crate) layout: wgpu::BindGroupLayout,
}

impl Default for Uniforms {
    fn default() -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
            default_view_proj: cgmath::Matrix4::identity().into(),
            time: now_millis(),
            padding: [0; 3],
        }
    }
}

impl OverlapBindGroup {
    pub fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                // Uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("Overlap Bind Group Layout"),
        });

        let uniforms = Uniforms::default();
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            uniforms,
            buffer,
            layout,
        }
    }

    pub fn update(&mut self, view_proj: ([[f32; 4]; 4], [[f32; 4]; 4])) {
        self.uniforms.view_proj = view_proj.0;
        self.uniforms.default_view_proj = view_proj.1;
        self.uniforms.time = now_millis();
    }

    pub fn update_buffer(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniforms]))
    }

    pub fn to_bind_group(
        &self,
        device: &wgpu::Device,
        overlap_depth_texture: &ScreenTexture,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&overlap_depth_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&overlap_depth_texture.sampler),
                },
            ],
            label: Some("Overlap Bind Group"),
        })
    }
}
