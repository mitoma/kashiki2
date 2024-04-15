use std::collections::BTreeMap;

use crate::{
    font_buffer::GlyphVertexBuffer,
    instances::{GlyphInstances, InstanceRaw},
    outline_bind_group::OutlineBindGroup,
    overlap_bind_group::OverlapBindGroup,
    screen_bind_group::ScreenBindGroup,
    screen_texture::{self, ScreenTexture},
    screen_vertex_buffer::ScreenVertexBuffer,
};

#[derive(Clone, Copy)]
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
pub(crate) struct RasterizerPipeline {
    // 1 ステージ目(overlap)
    pub(crate) overlap_bind_group: OverlapBindGroup,
    pub(crate) overlap_render_pipeline: wgpu::RenderPipeline,

    // 1 ステージ目のアウトプット(≒ 2 ステージ目のインプット)
    pub(crate) overlap_texture: ScreenTexture,

    // 2 ステージ目(outline)
    pub(crate) outline_bind_group: OutlineBindGroup,
    pub(crate) outline_render_pipeline: wgpu::RenderPipeline,
    pub(crate) outline_vertex_buffer: ScreenVertexBuffer,
    // 背景色。 2 ステージ目で使われる。
    pub(crate) bg_color: wgpu::Color,

    // 2 ステージ目のアウトプット(≒ 3 ステージ目のインプット)
    pub(crate) outline_texture: ScreenTexture,

    // 画面に表示する用のパイプライン
    pub(crate) screen_bind_group: ScreenBindGroup,
    pub(crate) screen_render_pipeline: wgpu::RenderPipeline,
    pub(crate) screen_vertex_buffer: ScreenVertexBuffer,
}

impl RasterizerPipeline {
    pub(crate) fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        screen_texture_format: wgpu::TextureFormat,
        quarity: Quarity,
        bg_color: wgpu::Color,
    ) -> Self {
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
                    buffers: &[GlyphVertexBuffer::desc(), InstanceRaw::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &overlap_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: overlap_texture.texture_format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent::REPLACE,
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
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
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
        let outline_vertex_buffer = ScreenVertexBuffer::new_buffer(device);

        // default screen render pipeline
        let screen_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Outline Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader/screen_shader.wgsl").into()),
        });

        let screen_bind_group = ScreenBindGroup::new(device);

        let screen_render_pipeline =
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
        let screen_vertex_buffer = ScreenVertexBuffer::new_buffer(device);

        Self {
            // overlap
            overlap_texture,
            overlap_bind_group,
            overlap_render_pipeline,
            // outline
            outline_texture,
            outline_bind_group,
            outline_render_pipeline,
            outline_vertex_buffer,
            bg_color,

            // default
            screen_render_pipeline,
            screen_bind_group,
            screen_vertex_buffer,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn run_all_stage(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        glyph_vertex_buffer: &GlyphVertexBuffer,
        view_proj: ([[f32; 4]; 4], [[f32; 4]; 4]),
        instances: &[&GlyphInstances],
        screen_view: wgpu::TextureView,
    ) {
        self.overlap_bind_group.update(view_proj);
        self.overlap_bind_group.update_buffer(queue);
        self.overlap_stage(encoder, glyph_vertex_buffer, instances);
        self.outline_stage(encoder, device);
        self.screen_stage(encoder, device, screen_view);
    }

    pub(crate) fn overlap_stage(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        glyph_vertex_buffer: &GlyphVertexBuffer,
        instances: &[&GlyphInstances],
    ) {
        let overlap_bind_group = &self.overlap_bind_group.bind_group;

        let mut instance_buffers = BTreeMap::new();
        for instance in instances.iter() {
            let instances = instance_buffers
                .entry((instance.c, instance.direction))
                .or_insert_with(Vec::new);
            instances.push((instance.len(), instance.to_wgpu_buffer()));
        }

        {
            let mut overlay_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Overlap Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.overlap_texture.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            overlay_render_pass.set_pipeline(&self.overlap_render_pipeline);
            overlay_render_pass.set_bind_group(0, overlap_bind_group, &[]);
            // vertex_buffer と index_buffer はほとんどの場合同一の buffer に収まると
            // 考えられるので ID を保持しておいて切り替え不要な場合には切り替えない。
            // 涙ぐましい最適化だがあまり効果がなさそうな気もするのでできればバッサリ消したい。
            // 微妙に意味があるかもしれないのでいったん残す。
            let mut vertex_buffer_id = None;
            let mut index_buffer_id = None;
            for ((c, direction), instances) in instance_buffers.iter() {
                for (len, buffer) in instances {
                    if let Ok(draw_info) = glyph_vertex_buffer.draw_info(c, direction) {
                        // グリフの座標情報(vertex)
                        if vertex_buffer_id != Some(draw_info.vertex.global_id()) {
                            overlay_render_pass.set_vertex_buffer(0, draw_info.vertex.slice(..));
                            vertex_buffer_id = Some(draw_info.vertex.global_id());
                        }
                        // グリフの座標情報(index)
                        if index_buffer_id != Some(draw_info.index.global_id()) {
                            overlay_render_pass.set_index_buffer(
                                draw_info.index.slice(..),
                                wgpu::IndexFormat::Uint32,
                            );
                            index_buffer_id = Some(draw_info.index.global_id());
                        }
                        // インスタンスの位置
                        overlay_render_pass.set_vertex_buffer(1, buffer.slice(..));
                        overlay_render_pass.draw_indexed(
                            draw_info.index_range.clone(),
                            0,
                            0..*len as _,
                        );
                    }
                }
            }
        }
    }

    pub(crate) fn outline_stage(&self, encoder: &mut wgpu::CommandEncoder, device: &wgpu::Device) {
        let outline_view = self
            .outline_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let outline_bind_group = self
            .outline_bind_group
            .to_bind_group(device, &self.overlap_texture);

        {
            let mut outline_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &outline_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.bg_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            outline_render_pass.set_pipeline(&self.outline_render_pipeline);
            outline_render_pass.set_bind_group(0, &outline_bind_group, &[]);
            outline_render_pass
                .set_vertex_buffer(0, self.outline_vertex_buffer.vertex_buffer.slice(..));
            outline_render_pass.set_index_buffer(
                self.outline_vertex_buffer.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            outline_render_pass.draw_indexed(
                self.outline_vertex_buffer.index_range.clone(),
                0,
                0..1,
            );
        }
    }

    pub(crate) fn screen_stage(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        screen_view: wgpu::TextureView,
    ) {
        let screen_bind_group = &self
            .screen_bind_group
            .to_bind_group(device, &self.outline_texture);
        {
            let mut screen_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Screen Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &screen_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            screen_render_pass.set_pipeline(&self.screen_render_pipeline);
            screen_render_pass.set_bind_group(0, screen_bind_group, &[]);
            screen_render_pass
                .set_vertex_buffer(0, self.screen_vertex_buffer.vertex_buffer.slice(..));
            screen_render_pass.set_index_buffer(
                self.screen_vertex_buffer.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            screen_render_pass.draw_indexed(self.screen_vertex_buffer.index_range.clone(), 0, 0..1);
        }
    }
}
