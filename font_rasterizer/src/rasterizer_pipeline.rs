use image::DynamicImage;
use wgpu::include_wgsl;

use crate::{
    background_bind_group::BackgroundImageBindGroup,
    glyph_instances::GlyphInstances,
    glyph_vertex_buffer::GlyphVertexBuffer,
    rasterizer_renderrer::RasterizerRenderrer,
    screen_bind_group::ScreenBindGroup,
    screen_texture::{BackgroundImageTexture, ScreenTexture},
    screen_vertex_buffer::ScreenVertexBuffer,
    vector_instances::VectorInstances,
    vector_vertex_buffer::VectorVertexBuffer,
};

const SCREEN_SHADER_DESCRIPTOR: wgpu::ShaderModuleDescriptor =
    include_wgsl!("shader/screen_shader.wgsl");
const BACKGROUND_IMAGE_SHADER_DESCRIPTOR: wgpu::ShaderModuleDescriptor =
    include_wgsl!("shader/screen_shader.wgsl");

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub enum Quarity {
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
    /// 固定クオリティの設定
    Fixed(u32, u32),
    /// 基本的に 2 倍サンプリングするが最大解像度を超えないようにする設定
    CappedVeryHigh(u32, u32),
}

pub struct Buffers<'a> {
    pub glyph_buffers: Option<(&'a GlyphVertexBuffer, &'a [&'a GlyphInstances])>,
    pub vector_buffers: Option<(
        &'a VectorVertexBuffer<String>,
        &'a [&'a VectorInstances<String>],
    )>,
}

/// フォントをラスタライズするためのパイプラインを提供する。
///
/// このブログの記事の内容を元に実装されている。
/// https://medium.com/@evanwallace/easy-scalable-text-rendering-on-the-gpu-c3f4d782c5ac
///
/// このパイプラインは 3 つのステージがある。
///
/// 1 つめはフォントを構成するポリゴンを重ねていく処理
/// 2 つめはポリゴンの重ねた結果からフォントの輪郭を抽出する処理
/// 3 つめは輪郭を抽出したテクスチャをスクリーンに描画する処理
///   2 が 3 よりも解像度が高ければオーバーサンプリングでクオリティが高くなり
///   その逆であればドット絵の品質になるよう調整
pub struct RasterizerPipeline {
    pub(crate) rasterizer_renderrer: RasterizerRenderrer,
    pub(crate) rasterizer_renderrer_for_modal: RasterizerRenderrer,

    // outline_texture を外部から渡すため、ここで保持する
    pub(crate) outline_texture: ScreenTexture,
    pub(crate) outline_texture_for_modal: ScreenTexture,

    // 背景色。
    pub bg_color: wgpu::Color,

    // バックグラウンド用のテクスチャ
    pub(crate) background_image_texture: Option<BackgroundImageTexture>,
    pub(crate) background_image_bind_group: BackgroundImageBindGroup,
    pub(crate) background_image_render_pipeline: wgpu::RenderPipeline,

    // 画面に表示する用のパイプライン
    pub(crate) screen_bind_group: ScreenBindGroup,
    pub(crate) screen_render_pipeline: wgpu::RenderPipeline,
    pub(crate) screen_render_modal_background_pipeline: wgpu::RenderPipeline,
    pub(crate) screen_vertex_buffer: ScreenVertexBuffer,
}

impl RasterizerPipeline {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        target_texture_format: wgpu::TextureFormat,
        quarity: Quarity,
        bg_color: wgpu::Color,
    ) -> Self {
        let enable_antialiasing = true;
        let (width, height) = match quarity {
            Quarity::VeryHigh => (width * 2, height * 2),
            Quarity::High => (width + width / 2, height + height / 2),
            Quarity::Middle => (width, height),
            Quarity::Low => (width - width / 4, height - height / 4),
            Quarity::VeryLow => (width / 2, height / 2),
            Quarity::Fixed(width, height) => (width, height),
            Quarity::CappedVeryHigh(capped_width, capped_height) => {
                let width = if width * 2 > capped_width {
                    capped_width
                } else {
                    width * 2
                };
                let height = if height * 2 > capped_height {
                    capped_height
                } else {
                    height * 2
                };
                (width, height)
            }
        };
        // GPU の上限によってはテクスチャのサイズを制限する
        let max = device.limits().max_texture_dimension_2d;
        let (width, height) = if width > max || height > max {
            if width > height {
                (max, height * max / width)
            } else {
                (width * max / height, max)
            }
        } else {
            (width, height)
        };

        let rasterizer_renderrer = RasterizerRenderrer::new(
            device,
            width,
            height,
            target_texture_format,
            enable_antialiasing,
        );
        let rasterizer_renderrer_for_modal = RasterizerRenderrer::new(
            device,
            width,
            height,
            target_texture_format,
            enable_antialiasing,
        );

        let outline_texture = ScreenTexture::new_with_format(
            device,
            (width, height),
            target_texture_format,
            Some("Outline Texture"),
        );
        let outline_texture_for_modal = ScreenTexture::new_with_format(
            device,
            (width, height),
            target_texture_format,
            Some("Outline Texture for Modal"),
        );

        let background_image_shader =
            device.create_shader_module(BACKGROUND_IMAGE_SHADER_DESCRIPTOR);
        let background_image_bind_group = BackgroundImageBindGroup::new(device);

        let background_image_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Background Image Render Pipeline Layout"),
                bind_group_layouts: &[&background_image_bind_group.layout],
                push_constant_ranges: &[],
            });

        let background_image_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Default Background Image Render Pipeline"),
                layout: Some(&background_image_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &background_image_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[ScreenVertexBuffer::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &background_image_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_texture_format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
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
                // render pipeline cache。起動時間の短縮に有利そうな気配だけどまぁ難しそうなので一旦無しで。
                cache: None,
            });

        // default screen render pipeline
        let screen_shader = device.create_shader_module(SCREEN_SHADER_DESCRIPTOR);

        let screen_bind_group = ScreenBindGroup::new(device);

        let screen_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Screen Render Pipeline Layout"),
                bind_group_layouts: &[&screen_bind_group.layout],
                push_constant_ranges: &[],
            });

        let screen_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Default Screen Render Pipeline"),
                layout: Some(&screen_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &screen_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[ScreenVertexBuffer::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &screen_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_texture_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
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
                // render pipeline cache。起動時間の短縮に有利そうな気配だけどまぁ難しそうなので一旦無しで。
                cache: None,
            });

        let screen_render_modal_background_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Default Screen Render Modal Background Pipeline"),
                layout: Some(&screen_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &screen_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[ScreenVertexBuffer::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &screen_shader,
                    entry_point: Some("fs_main_modal_background"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_texture_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
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
                // render pipeline cache。起動時間の短縮に有利そうな気配だけどまぁ難しそうなので一旦無しで。
                cache: None,
            });

        let screen_vertex_buffer = ScreenVertexBuffer::new_buffer(device);

        Self {
            rasterizer_renderrer,
            rasterizer_renderrer_for_modal,
            outline_texture,
            outline_texture_for_modal,
            bg_color,

            // バックグラウンド
            background_image_texture: None,
            background_image_bind_group,
            background_image_render_pipeline,

            // default
            screen_render_pipeline,
            screen_render_modal_background_pipeline,
            screen_bind_group,
            screen_vertex_buffer,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_all_stage(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view_proj: ([[f32; 4]; 4], [[f32; 4]; 4]),
        buffers: Buffers,
        modal_buffers: Buffers,
        screen_view: wgpu::TextureView,
    ) {
        let has_modal_background =
            modal_buffers.glyph_buffers.is_some() || modal_buffers.vector_buffers.is_some();

        self.rasterizer_renderrer.prepare(device, queue, view_proj);
        self.rasterizer_renderrer
            .render(encoder, buffers, &self.outline_texture.view);

        if has_modal_background {
            self.rasterizer_renderrer_for_modal
                .prepare(device, queue, view_proj);
            self.rasterizer_renderrer_for_modal.render(
                encoder,
                modal_buffers,
                &self.outline_texture_for_modal.view,
            );
        }

        self.screen_background_image_stage(encoder, device, &screen_view);
        self.screen_stage(encoder, device, screen_view, has_modal_background);
    }

    pub(crate) fn screen_stage(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        screen_view: wgpu::TextureView,
        has_modal_background: bool,
    ) {
        let screen_bind_group = &self
            .screen_bind_group
            .to_bind_group(device, &self.outline_texture);
        let screen_bind_group_for_modal = &self
            .screen_bind_group
            .to_bind_group(device, &self.outline_texture_for_modal);
        {
            let mut screen_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Screen Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &screen_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if has_modal_background {
                screen_render_pass.set_pipeline(&self.screen_render_modal_background_pipeline);
            } else {
                screen_render_pass.set_pipeline(&self.screen_render_pipeline);
            }
            screen_render_pass.set_bind_group(0, screen_bind_group, &[]);
            screen_render_pass
                .set_vertex_buffer(0, self.screen_vertex_buffer.vertex_buffer.slice(..));
            screen_render_pass.set_index_buffer(
                self.screen_vertex_buffer.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            screen_render_pass.draw_indexed(self.screen_vertex_buffer.index_range.clone(), 0, 0..1);

            if has_modal_background {
                screen_render_pass.set_pipeline(&self.screen_render_pipeline);
                screen_render_pass.set_bind_group(0, screen_bind_group_for_modal, &[]);
                screen_render_pass
                    .set_vertex_buffer(0, self.screen_vertex_buffer.vertex_buffer.slice(..));
                screen_render_pass.set_index_buffer(
                    self.screen_vertex_buffer.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint16,
                );
                screen_render_pass.draw_indexed(
                    self.screen_vertex_buffer.index_range.clone(),
                    0,
                    0..1,
                );
            }
        }
    }

    pub(crate) fn screen_background_image_stage(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        screen_view: &wgpu::TextureView,
    ) {
        let mut screen_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Screen Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: screen_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.bg_color),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        let Some(background_image_texture) = self.background_image_texture.as_ref() else {
            return;
        };

        let screen_bind_group = &self
            .background_image_bind_group
            .to_bind_group(device, background_image_texture);

        screen_render_pass.set_pipeline(&self.background_image_render_pipeline);
        screen_render_pass.set_bind_group(0, screen_bind_group, &[]);
        screen_render_pass.set_vertex_buffer(0, self.screen_vertex_buffer.vertex_buffer.slice(..));
        screen_render_pass.set_index_buffer(
            self.screen_vertex_buffer.index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );
        screen_render_pass.draw_indexed(self.screen_vertex_buffer.index_range.clone(), 0, 0..1);
    }

    pub fn set_background_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        background_image: Option<&DynamicImage>,
    ) {
        self.background_image_texture = background_image.map(|image| {
            BackgroundImageTexture::new(device, queue, image, Some("Background Image Texture"))
        });
    }

    // render_state から呼び出す用。prepare という名前が適切か？
    #[inline]
    pub fn update_buffer(&mut self, queue: &wgpu::Queue) {
        self.rasterizer_renderrer
            .overlap_bind_group
            .update_buffer(queue);
    }
}
