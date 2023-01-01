use crate::{
    font_vertex_buffer::FontVertexBuffer,
    instances::InstanceRaw,
    outline_bind_group::OutlineBindGroup,
    overlap_bind_group::OverlapBindGroup,
    screen_bind_group::ScreenBindGroup,
    screen_texture::{self, ScreenTexture},
    screen_vertex_buffer::ScreenVertexBuffer,
};

pub(crate) enum Quarity {
    /// 2 倍サンプリングする(アンチエイリアスあり)
    VeryHigh,
    /// 1.5 倍サンプリングする(アンチエイリアスあり)
    High,
    /// アンリエイリアスしない(アンチエイリアスなし)
    Middle,
    /// 0.75倍サンプリング
    Low,
    /// 0.5倍サンプリング
    VeryLow,
}

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
    pub(crate) overlap_bind_group: OverlapBindGroup,
    pub(crate) overlap_render_pipeline: wgpu::RenderPipeline,
    pub(crate) overlap_texture: ScreenTexture,
    // 2 ステージ目(outline)
    pub(crate) outline_bind_group: OutlineBindGroup,
    pub(crate) outline_render_pipeline: wgpu::RenderPipeline,
    pub(crate) outline_texture: ScreenTexture,

    // default screen pipeline
    // 画面に表示する用のパイプライン
    pub(crate) default_screen_render_pipeline: wgpu::RenderPipeline,
    pub(crate) default_screen_bind_group: ScreenBindGroup,
}

impl RasterizerPipeline {
    pub(crate) fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        screen_texture_format: wgpu::TextureFormat,
        quarity: Quarity,
    ) -> Self {
        let (width, height) = match quarity {
            Quarity::VeryHigh => (width * 2, height * 2),
            Quarity::High => (width + width / 2, height + height / 2),
            Quarity::Middle => (width, height),
            Quarity::Low => (width - width / 4, height - height / 4),
            Quarity::VeryLow => (width / 2, height / 2),
        };

        // overlap
        let overlap_texture =
            screen_texture::ScreenTexture::new(device, (width, height), Some("Overlap Texture"));

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
                    buffers: &[FontVertexBuffer::desc(), InstanceRaw::desc()],
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
            screen_texture::ScreenTexture::new(device, (width, height), Some("Outline Texture"));

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
                    buffers: &[ScreenVertexBuffer::desc()],
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

        // default screen render pipeline
        let screen_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Outline Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader/screen_shader.wgsl").into()),
        });

        let default_screen_bind_group = ScreenBindGroup::new(device);

        let default_screen_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Default Screen Render Pipeline"),
                layout: Some(&outline_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &screen_shader,
                    entry_point: "vs_main",
                    buffers: &[ScreenVertexBuffer::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &screen_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: screen_texture_format,
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
            overlap_texture,
            overlap_bind_group,
            overlap_render_pipeline,
            // outline
            outline_texture,
            outline_bind_group,
            outline_render_pipeline,

            // default
            default_screen_render_pipeline,
            default_screen_bind_group,
        }
    }
}
