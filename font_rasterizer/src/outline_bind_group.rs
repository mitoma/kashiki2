use crate::screen_texture::{ScreenTexture, TXAA_TEXTURE_FORMAT, TxaaTexture};

/// アウトライン用の BindGroup。
/// Overlay 情報の書き込まれた Texture と Sampler のみを受け取る。
///
/// テクスチャの RGBA の意味
/// R, G, B: 色情報
/// A: 重ね合わせの数
pub struct OutlineBindGroup {
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
            ],
            label: Some("Outline Bind Group Layout"),
        });
        Self { layout }
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
            ],
            label: Some("Outline Bind Group"),
        })
    }
}
