use crate::{overlap_bind_group::OverlapBindGroup, rasterizer_pipeline::Buffers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlineFillRule {
    EvenOdd,
    NonZero,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum RenderingBackend {
    #[default]
    Legacy,
    Windfoil,
}

enum Backend {
    Legacy(Box<crate::legacy_renderrer::RasterizerRenderrer>),
    Windfoil(Box<crate::windfoil_renderrer::RasterizerRenderrer>),
}

pub struct RasterizerRenderrer {
    backend: Backend,
}

impl RasterizerRenderrer {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        target_texture_format: wgpu::TextureFormat,
        enable_antialiasing: bool,
        outline_fill_rule: OutlineFillRule,
    ) -> Self {
        Self::new_with_backend(
            device,
            width,
            height,
            target_texture_format,
            enable_antialiasing,
            outline_fill_rule,
            RenderingBackend::Legacy,
        )
    }

    pub fn new_with_backend(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        target_texture_format: wgpu::TextureFormat,
        enable_antialiasing: bool,
        outline_fill_rule: OutlineFillRule,
        backend: RenderingBackend,
    ) -> Self {
        let backend = match backend {
            RenderingBackend::Legacy => {
                Backend::Legacy(Box::new(crate::legacy_renderrer::RasterizerRenderrer::new(
                    device,
                    width,
                    height,
                    target_texture_format,
                    enable_antialiasing,
                    outline_fill_rule,
                )))
            }
            RenderingBackend::Windfoil => Backend::Windfoil(Box::new(
                crate::windfoil_renderrer::RasterizerRenderrer::new(
                    device,
                    width,
                    height,
                    target_texture_format,
                    enable_antialiasing,
                    outline_fill_rule,
                ),
            )),
        };
        Self { backend }
    }

    pub fn new_with_curve_projection(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        target_texture_format: wgpu::TextureFormat,
        enable_antialiasing: bool,
        outline_fill_rule: OutlineFillRule,
        preproject_curves: bool,
    ) -> Self {
        Self {
            backend: Backend::Windfoil(Box::new(
                crate::windfoil_renderrer::RasterizerRenderrer::new_with_curve_projection(
                    device,
                    width,
                    height,
                    target_texture_format,
                    enable_antialiasing,
                    outline_fill_rule,
                    preproject_curves,
                ),
            )),
        }
    }

    pub(crate) fn overlap_bind_group(&mut self) -> &mut OverlapBindGroup {
        match &mut self.backend {
            Backend::Legacy(renderer) => &mut renderer.overlap_bind_group,
            Backend::Windfoil(renderer) => &mut renderer.overlap_bind_group,
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view_proj: ([[f32; 4]; 4], [[f32; 4]; 4]),
    ) {
        match &mut self.backend {
            Backend::Legacy(renderer) => renderer.prepare(device, queue, view_proj),
            Backend::Windfoil(renderer) => renderer.prepare(device, queue, view_proj),
        }
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        buffers: Buffers,
        target_view: &wgpu::TextureView,
    ) {
        match &self.backend {
            Backend::Legacy(renderer) => renderer.render(encoder, buffers, target_view),
            Backend::Windfoil(renderer) => renderer.render(encoder, buffers, target_view),
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::{
        VectorVertexBuilder,
        vector_instances::{InstanceAttributes, VectorInstances},
        vector_vertex_buffer::VectorVertexBuffer,
    };

    #[test]
    #[ignore = "requires a WebGPU adapter"]
    fn rendering_backends_switch_with_shared_buffers() {
        pollster::block_on(async {
            let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
            if cfg!(target_os = "windows") {
                descriptor.backends = wgpu::Backends::DX12;
            }
            let instance = wgpu::Instance::new(descriptor);
            let adapter = instance.request_adapter(&Default::default()).await.unwrap();
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    required_features: adapter.features()
                        & wgpu::Features::CONSERVATIVE_RASTERIZATION,
                    ..Default::default()
                })
                .await
                .unwrap();
            let mut builder = VectorVertexBuilder::new();
            builder.move_to(-0.5, -0.5);
            builder.line_to(0.5, -0.5);
            builder.line_to(0.5, 0.5);
            builder.line_to(-0.5, 0.5);
            builder.close();
            let key = "rectangle".to_string();
            let mut vertices = VectorVertexBuffer::new();
            vertices
                .append(&device, &queue, key.clone(), builder.build())
                .unwrap();
            let mut instances = VectorInstances::new(key, &device);
            instances.push(InstanceAttributes {
                position: glam::Vec3::ZERO,
                color: [1.0; 3],
                start_time: 0,
                ..Default::default()
            });
            instances.update_buffer(&device, &queue);
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Backend switch test"),
                size: wgpu::Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            let output = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Backend switch readback"),
                size: 64 * 64 * 4,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            for rule in [OutlineFillRule::NonZero, OutlineFillRule::EvenOdd] {
                let mut reference = [None, None];
                for backend in [
                    RenderingBackend::Legacy,
                    RenderingBackend::Windfoil,
                    RenderingBackend::Legacy,
                    RenderingBackend::Windfoil,
                ] {
                    let mut renderer = RasterizerRenderrer::new_with_backend(
                        &device,
                        64,
                        64,
                        texture.format(),
                        true,
                        rule,
                        backend,
                    );
                    let identity = glam::Mat4::IDENTITY.to_cols_array_2d();
                    renderer.prepare(&device, &queue, (identity, identity));
                    let mut encoder = device.create_command_encoder(&Default::default());
                    renderer.render(
                        &mut encoder,
                        Buffers {
                            glyph_buffers: None,
                            vector_buffers: Some((&vertices, &[&instances])),
                        },
                        &view,
                    );
                    encoder.copy_texture_to_buffer(
                        texture.as_image_copy(),
                        wgpu::TexelCopyBufferInfo {
                            buffer: &output,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(256),
                                rows_per_image: Some(64),
                            },
                        },
                        texture.size(),
                    );
                    queue.submit([encoder.finish()]);
                    let (sender, receiver) = std::sync::mpsc::channel();
                    output
                        .slice(..)
                        .map_async(wgpu::MapMode::Read, move |result| {
                            sender.send(result).unwrap()
                        });
                    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                    receiver.recv().unwrap().unwrap();
                    let pixels = output.slice(..).get_mapped_range().unwrap().to_vec();
                    output.unmap();
                    assert_eq!(pixels[(30 * 64 + 25) * 4 + 3], 255, "{backend:?}");
                    assert_eq!(pixels[3], 0, "{backend:?}");
                    let slot = usize::from(backend == RenderingBackend::Windfoil);
                    if let Some(expected) = &reference[slot] {
                        assert_eq!(&pixels, expected, "{backend:?}");
                    } else {
                        reference[slot] = Some(pixels);
                    }
                }
            }
        });
    }
}
