use std::time::SystemTime;

use wgpu::util::DeviceExt;

use crate::{
    font_vertex::FontVertex,
    screen_texture::{self, ScreenTexture},
};

/// フォントをラスタライズするためのパイプラインを提供する。
///
/// このブログの記事の内容を元に実装されている。
/// https://medium.com/@evanwallace/easy-scalable-text-rendering-on-the-gpu-c3f4d782c5ac
///
/// このパイプラインは 2 つのステージがある。
///
/// 1 つめはフォントを構成するポリゴンを重ねていく処理
/// 2 つめはポリゴンの重ねた結果からフォントの輪郭を抽出する処理
pub(crate) struct RasterizerPipeline {
    // 1 ステージ目(overlap)
    overlap_bind_group: OverlapBindGroup,
    overlap_render_pipeline_layout: wgpu::PipelineLayout,
    overlap_render_pipeline: wgpu::RenderPipeline,
    overlap_texture: ScreenTexture,
    // 2 ステージ目(outline)
    outline_bind_group: OutlineBindGroup,
    outline_render_pipeline_layout: wgpu::PipelineLayout,
    outline_render_pipeline: wgpu::RenderPipeline,
    outline_texture: ScreenTexture,
}

impl RasterizerPipeline {
    pub(crate) fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        // overlap
        let overlap_texture =
            screen_texture::ScreenTexture::new(&device, (width, height), Some("Overlap Texture"));

        let overlap_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Overlap Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader/overlap_shader.wgsl").into()),
        });

        let overlap_bind_group = OverlapBindGroup::new(device);

        let overlap_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Overlap Render Pipeline Layout"),
                bind_group_layouts: &[&overlap_bind_group.layout],
                push_constant_ranges: &[],
            });

        let overlap_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Overlap Render Pipeline"),
                layout: Some(&overlap_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &overlap_shader,
                    entry_point: "vs_main",
                    buffers: &[FontVertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &overlap_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: overlap_texture.texture_format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None, // 字に表裏はあまり関係ないのでカリングはしない
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: false,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                // If the pipeline will be used with a multiview render pass, this
                // indicates how many array layers the attachments will have.
                multiview: None,
            });

        // outline
        let outline_texture =
            screen_texture::ScreenTexture::new(&device, (width, height), Some("Outline Texture"));

        let outline_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Outline Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader/outline_shader.wgsl").into()),
        });

        let outline_bind_group = OutlineBindGroup::new(device);

        let outline_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Outline Render Pipeline Layout"),
                bind_group_layouts: &[&outline_bind_group.layout],
                push_constant_ranges: &[],
            });

        let outline_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Outline Render Pipeline"),
                layout: Some(&outline_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &outline_shader,
                    entry_point: "vs_main",
                    buffers: &[ScreenVertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &outline_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: outline_texture.texture_format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent::REPLACE,
                            alpha: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                    // or Features::POLYGON_MODE_POINT
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: false,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                // If the pipeline will be used with a multiview render pass, this
                // indicates how many array layers the attachments will have.
                multiview: None,
            });

        Self {
            // overlap
            overlap_bind_group,
            overlap_texture,
            overlap_render_pipeline_layout,
            overlap_render_pipeline,
            // outline
            outline_texture,
            outline_bind_group,
            outline_render_pipeline_layout,
            outline_render_pipeline,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ScreenVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl ScreenVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
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
}

const SCREEN_VERTICES: &[ScreenVertex] = &[
    ScreenVertex {
        position: [-1.0, -1.0, 0.0],
        tex_coords: [0.0, 1.0],
    }, // A
    ScreenVertex {
        position: [1.0, -1.0, 0.0],
        tex_coords: [1.0, 1.0],
    }, // B
    ScreenVertex {
        position: [-1.0, 1.0, 0.0],
        tex_coords: [0.0, 0.0],
    }, // C
    ScreenVertex {
        position: [1.0, 1.0, 0.0],
        tex_coords: [1.0, 0.0],
    }, // D
];

const SCREEN_INDICES: &[u16] = &[0, 1, 2, 2, 1, 3];

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    time: f32,
}

pub struct OverlapBindGroup {
    uniforms: Uniforms,
    layout: wgpu::BindGroupLayout,
}

impl OverlapBindGroup {
    pub fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Overlap Bind Group Layout"),
        });

        let uniforms = Uniforms {
            time: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as f32
                / 100.0,
        };

        Self { uniforms, layout }
    }

    pub fn update(&mut self) {
        let d = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        self.uniforms.time = (d.as_millis() % 10000000) as f32 / 1000.0;
    }

    pub fn to_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.to_wgpu_buffer(device).as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        })
    }

    fn to_wgpu_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[self.uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }
}

pub struct OutlineBindGroup {
    layout: wgpu::BindGroupLayout,
}

impl OutlineBindGroup {
    fn new(device: &wgpu::Device) -> Self {
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
