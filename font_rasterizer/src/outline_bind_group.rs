use wgpu::util::DeviceExt;

use crate::screen_texture::{ScreenTexture, TXAA_TEXTURE_FORMAT, TxaaTexture};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    frame_count: u32,
    // padding が必要らしい。
    padding: [u32; 3],
}

impl Default for Uniforms {
    fn default() -> Self {
        Self {
            frame_count: 0,
            padding: [0; 3],
        }
    }
}

/// アウトライン用の BindGroup。
/// Overlay 情報の書き込まれた Texture と Sampler のみを受け取る。
///
/// テクスチャの RGBA の意味
/// R, G, B: 色情報
/// A: 重ね合わせの数
pub struct OutlineBindGroup {
    uniforms: Uniforms,
    buffer: wgpu::Buffer,
    pub(crate) layout: wgpu::BindGroupLayout,
}

impl OutlineBindGroup {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                // Overlay 情報の書き込まれたテクスチャ
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
                // サンプラー
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // temporal anti aliasing 用のテクスチャ
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::ReadWrite,
                        format: TXAA_TEXTURE_FORMAT,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("Outline Bind Group Layout"),
        });
        let uniforms = Uniforms::default();
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

    pub fn update(&mut self) {
        self.uniforms.frame_count += 1;
    }

    pub fn update_buffer(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniforms]))
    }

    pub fn to_bind_group(
        &self,
        device: &wgpu::Device,
        overlap_texture: &ScreenTexture,
        aa_texture: &TxaaTexture,
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
                    resource: wgpu::BindingResource::TextureView(&aa_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.buffer.as_entire_binding(),
                },
            ],
            label: Some("Outline Bind Group"),
        })
    }
}
