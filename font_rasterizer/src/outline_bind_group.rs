use wgpu::util::DeviceExt;

use crate::{overlap_record_texture::OverlapRecordBuffer, screen_texture::ScreenTexture};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    width: u32,
    // padding が必要らしい。正直意味わかんねぇな。
    padding: [u32; 3],
}

impl Default for Uniforms {
    fn default() -> Self {
        Self {
            width: 0,
            padding: [0; 3],
        }
    }
}

/// オーバーラップ用の BindGroup。
/// Uniforms として現在時刻のみ渡している。
pub struct OutlineBindGroup {
    uniforms: Uniforms,
    buffer: wgpu::Buffer,
    pub(crate) layout: wgpu::BindGroupLayout,
}

impl OutlineBindGroup {
    pub fn new(device: &wgpu::Device, width: u32) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // Uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("Outline Bind Group Layout"),
        });

        let uniforms = Uniforms {
            width,
            ..Default::default()
        };
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Outline Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            uniforms,
            buffer,
            layout,
        }
    }

    pub fn to_bind_group(
        &self,
        device: &wgpu::Device,
        overlap_texture: &ScreenTexture,
        overlap_record_buffer: &OverlapRecordBuffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&overlap_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&overlap_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: overlap_record_buffer.buffer.as_entire_binding(),
                },
            ],
            label: Some("Outline Bind Group"),
        })
    }
}
