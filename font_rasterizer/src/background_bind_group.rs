use crate::screen_texture::BackgroundImageTexture;

/// Screen用の BindGroup。
/// Outline の Texture をアンチエイリアスする
pub struct BackgroundImageBindGroup {
    pub(crate) layout: wgpu::BindGroupLayout,
}

impl BackgroundImageBindGroup {
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
            label: Some("Background Image Bind Group Layout"),
        });
        Self { layout }
    }

    pub fn to_bind_group(
        &self,
        device: &wgpu::Device,
        background_image_texture: &BackgroundImageTexture,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&background_image_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&background_image_texture.sampler),
                },
            ],
            label: Some("Background Image Bind Group"),
        })
    }
}
