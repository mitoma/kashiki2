use std::{collections::BTreeMap, fs};

use wgpu::include_wgsl;

use crate::{
    debug_mode::DEBUG_FLAGS,
    glyph_instances::GlyphInstances,
    glyph_vertex_buffer::GlyphVertexBuffer,
    outline_bind_group::OutlineBindGroup,
    overlap_bind_group::OverlapBindGroup,
    rasterizer_pipeline::Buffers,
    screen_texture::ScreenTexture,
    screen_vertex_buffer::ScreenVertexBuffer,
    vector_instances::{InstanceRaw, VectorInstances},
    vector_vertex::Vertex,
    vector_vertex_buffer::VectorVertexBuffer,
};

const OVERLAP_SHADER_DESCRIPTOR: wgpu::ShaderModuleDescriptor =
    include_wgsl!("shader/overlap_shader.wgsl");
const OUTLINE_SHADER_DESCRIPTOR: wgpu::ShaderModuleDescriptor =
    include_wgsl!("shader/outline_shader.wgsl");

pub struct RasterizerRenderrer {
    // 1 ステージ目(overlap)
    pub(crate) overlap_bind_group: OverlapBindGroup,
    pub(crate) overlap_render_pipeline: wgpu::RenderPipeline,

    // 1 ステージ目のアウトプット(≒ 2 ステージ目のインプット)
    pub(crate) overlap_texture: ScreenTexture,
    // 重なり回数記録用のテクスチャ（マルチターゲット用）
    pub(crate) overlap_count_texture: ScreenTexture,

    pub(crate) outline_bind_group: OutlineBindGroup,
    pub(crate) outline_render_pipeline: wgpu::RenderPipeline,
    pub(crate) outline_vertex_buffer: ScreenVertexBuffer,

    // 2 ステージ目のアウトプット(≒ 3 ステージ目のインプット)
    pub(crate) outline_texture: ScreenTexture,
}
impl RasterizerRenderrer {
    /// Create all unchanging resources here.
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        // overlap
        let overlap_shader = if DEBUG_FLAGS.debug_shader {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("font_rasterizer/src/shader/overlap_shader.debug.wgsl"),
                source: wgpu::ShaderSource::Wgsl(
                    fs::read_to_string("font_rasterizer/src/shader/overlap_shader.debug.wgsl")
                        .unwrap()
                        .into(),
                ),
            })
        } else {
            device.create_shader_module(OVERLAP_SHADER_DESCRIPTOR)
        };

        let overlap_texture = ScreenTexture::new(device, (width, height), Some("Overlap Texture"));

        // 重なり回数記録用のテクスチャ（RGBA8Unorm フォーマットを使用してブレンド可能にする）
        let overlap_count_texture = ScreenTexture::new_with_format(
            device,
            (width, height),
            wgpu::TextureFormat::Bgra8Unorm,
            Some("Overlap Count Texture"),
        );

        let overlap_bind_group = OverlapBindGroup::new(device, width);

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
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::desc(), InstanceRaw::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &overlap_shader,
                    entry_point: Some("fs_main"),
                    targets: &[
                        Some(wgpu::ColorTargetState {
                            format: overlap_texture.texture_format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        Some(wgpu::ColorTargetState {
                            format: overlap_count_texture.texture_format,
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::One,
                                    dst_factor: wgpu::BlendFactor::One,
                                    operation: wgpu::BlendOperation::Add,
                                },
                                alpha: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::One,
                                    dst_factor: wgpu::BlendFactor::One,
                                    operation: wgpu::BlendOperation::Add,
                                },
                            }),
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                    ],
                    compilation_options: Default::default(),
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
                    conservative: true,
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

        // outline
        let outline_shader = if DEBUG_FLAGS.debug_shader {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("font_rasterizer/src/shader/outline_shader_debug.wgsl"),
                source: wgpu::ShaderSource::Wgsl(
                    fs::read_to_string("font_rasterizer/src/shader/outline_shader.debug.wgsl")
                        .unwrap()
                        .into(),
                ),
            })
        } else {
            device.create_shader_module(OUTLINE_SHADER_DESCRIPTOR)
        };

        let outline_texture = ScreenTexture::new(device, (width, height), Some("Outline Texture"));
        let outline_bind_group = OutlineBindGroup::new(device, width);
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
                    entry_point: Some("vs_main"),
                    buffers: &[ScreenVertexBuffer::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &outline_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: outline_texture.texture_format,
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
                    conservative: true,
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
        let outline_vertex_buffer = ScreenVertexBuffer::new_buffer(device);

        Self {
            overlap_bind_group,
            overlap_render_pipeline,
            overlap_texture,
            overlap_count_texture,
            outline_bind_group,
            outline_render_pipeline,
            outline_vertex_buffer,
            outline_texture,
        }
    }

    #[inline]
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view_proj: ([[f32; 4]; 4], [[f32; 4]; 4]),
        buffers: Buffers,
    ) {
        self.overlap_bind_group.update(view_proj);
        self.overlap_bind_group.update_buffer(queue);
        self.overlap_stage(encoder, buffers.glyph_buffers, buffers.vector_buffers);
        self.outline_stage(encoder, device);
    }

    #[inline]
    fn overlap_stage(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        glyph_buffers: Option<(&GlyphVertexBuffer, &[&GlyphInstances])>,
        vector_buffers: Option<(&VectorVertexBuffer<String>, &[&VectorInstances<String>])>,
    ) {
        let overlap_bind_group = &self.overlap_bind_group.bind_group;
        let mut overlay_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Overlap Render Pass"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
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
                    depth_slice: None,
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.overlap_count_texture.view,
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
                    depth_slice: None,
                }),
            ],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        overlay_render_pass.set_pipeline(&self.overlap_render_pipeline);
        overlay_render_pass.set_bind_group(0, overlap_bind_group, &[]);

        // そんなに重要ではないがベクターを先に描画した方が、文字色が不自然にならないので先にベクターを描画する（キャレットはベクターのため）
        // 文字以外のベクター画像の描画。文字のようにインスタンスが多い訳ではない(多くの場合は 1 つ)ので、それほど効率化はしていない。
        if let Some((vector_vertex_buffer, vector_instance_buffers)) = vector_buffers {
            for instance in vector_instance_buffers {
                let instances = instance.to_wgpu_buffer();
                if let Ok(draw_info) = vector_vertex_buffer.draw_info(&instance.key) {
                    overlay_render_pass.set_vertex_buffer(0, draw_info.vertex.slice(..));
                    overlay_render_pass
                        .set_index_buffer(draw_info.index.slice(..), wgpu::IndexFormat::Uint32);
                    overlay_render_pass.set_vertex_buffer(1, instances.slice(..));
                    overlay_render_pass.draw_indexed(
                        draw_info.index_range.clone(),
                        0,
                        0..instance.len() as _,
                    );
                }
            }
        }

        if let Some((glyph_vertex_buffer, glyph_instance_buffers)) = glyph_buffers {
            let mut instance_buffers = BTreeMap::new();
            for instance in glyph_instance_buffers {
                let instances = instance_buffers
                    .entry((instance.c, instance.direction))
                    .or_insert_with(Vec::new);
                instances.push((instance.len(), instance.to_wgpu_buffer()));
            }

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
                        if vertex_buffer_id != Some(draw_info.vertex) {
                            overlay_render_pass.set_vertex_buffer(0, draw_info.vertex.slice(..));
                            vertex_buffer_id = Some(draw_info.vertex);
                        }
                        // グリフの座標情報(index)
                        if index_buffer_id != Some(draw_info.index) {
                            overlay_render_pass.set_index_buffer(
                                draw_info.index.slice(..),
                                wgpu::IndexFormat::Uint32,
                            );
                            index_buffer_id = Some(draw_info.index);
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

    fn outline_stage(&self, encoder: &mut wgpu::CommandEncoder, device: &wgpu::Device) {
        let outline_view = self
            .outline_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let outline_bind_group = self.outline_bind_group.to_bind_group(
            device,
            &self.overlap_texture,
            &self.overlap_count_texture,
        );

        {
            let mut outline_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &outline_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
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
}
