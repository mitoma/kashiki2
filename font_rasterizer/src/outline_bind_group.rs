use crate::screen_texture::ScreenTexture;

/// アウトライン用の BindGroup。
/// Overlay 情報の書き込まれた Texture と Sampler のみを受け取る。
///
/// テクスチャの RGBA の意味
/// R: 重ね合わせの数
/// G: 有用な情報なし
/// B: 有用な情報なし
/// A: 透明度情報(意味があるかどうか不明)
pub struct OutlineBindGroup {
    pub(crate) layout: wgpu::BindGroupLayout,
}

impl OutlineBindGroup {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
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
            ],
            label: Some("Outline Bind Group Layout"),
        });
        Self { layout }
    }

    pub fn to_bind_group(
        &self,
        device: &wgpu::Device,
        overlap_texture: &ScreenTexture,
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
            ],
            label: Some("Outline Bind Group"),
        })
    }
}
