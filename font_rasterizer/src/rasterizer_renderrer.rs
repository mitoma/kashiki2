use std::fs;

use crate::{
    debug_mode::DEBUG_FLAGS, overlap_bind_group::OverlapBindGroup, rasterizer_pipeline::Buffers,
    screen_bind_group::ScreenBindGroup, screen_texture::ScreenTexture,
    screen_vertex_buffer::ScreenVertexBuffer, vector_instances::InstanceRaw, windfoil,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlineFillRule {
    EvenOdd,
    NonZero,
}

pub struct RasterizerRenderrer {
    enable_antialiasing: bool,
    pub(crate) overlap_bind_group: OverlapBindGroup,
    path_pipeline: wgpu::RenderPipeline,
    color_texture: ScreenTexture,
    resolve_bind_group: wgpu::BindGroup,
    resolve_pipeline: wgpu::RenderPipeline,
    resolve_vertices: ScreenVertexBuffer,
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
        let path_shader = if DEBUG_FLAGS.debug_shader {
            fs::read_to_string("font_rasterizer/src/shader/windfoil.wgsl").unwrap()
        } else {
            include_str!("shader/windfoil.wgsl").to_owned()
        };
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Windfoil"),
            source: wgpu::ShaderSource::Wgsl(path_shader.into()),
        });
        let overlap_bind_group = OverlapBindGroup::new(device, width, height);
        let paths_layout = windfoil::layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Windfoil pipeline layout"),
            bind_group_layouts: &[Some(&overlap_bind_group.layout), Some(&paths_layout)],
            immediate_size: 0,
        });
        let color_texture = ScreenTexture::new_with_format(
            device,
            (width, height),
            wgpu::TextureFormat::Rgba16Float,
            Some("Windfoil premultiplied color"),
        );
        let path_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Windfoil paths"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_windfoil"),
                buffers: &[Some(InstanceRaw::desc())],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(match outline_fill_rule {
                    OutlineFillRule::EvenOdd => "fs_windfoil_even_odd",
                    OutlineFillRule::NonZero => "fs_windfoil_non_zero",
                }),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_texture.texture_format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let resolve_layout = ScreenBindGroup::new(device);
        let resolve_bind_group = resolve_layout.to_bind_group(device, &color_texture);
        let resolve_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Windfoil straight-alpha resolve"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    include_str!("shader/screen_shader.wgsl"),
                    "@fragment fn fs_resolve(input: VertexOutput) -> @location(0) vec4<f32> {\n\
                 let color = textureSample(t_diffuse, s_diffuse, input.tex_coords);\n\
                 return vec4<f32>(color.rgb / max(color.a, 1e-8), color.a);\n}"
                )
                .into(),
            ),
        });
        let resolve_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Windfoil resolve layout"),
                bind_group_layouts: &[Some(&resolve_layout.layout)],
                immediate_size: 0,
            });
        let resolve_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Windfoil resolve"),
            layout: Some(&resolve_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &resolve_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(ScreenVertexBuffer::desc())],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &resolve_shader,
                entry_point: Some("fs_resolve"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_texture_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        Self {
            enable_antialiasing,
            overlap_bind_group,
            path_pipeline,
            color_texture,
            resolve_bind_group,
            resolve_pipeline,
            resolve_vertices: ScreenVertexBuffer::new_buffer(device),
        }
    }

    pub fn prepare(
        &mut self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        view_proj: ([[f32; 4]; 4], [[f32; 4]; 4]),
    ) {
        self.overlap_bind_group
            .update(view_proj, self.enable_antialiasing as u32);
        self.overlap_bind_group.update_buffer(queue);
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        buffers: Buffers,
        target_view: &wgpu::TextureView,
    ) {
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Windfoil paths"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.color_texture.view,
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
                multiview_mask: None,
            });
            pass.set_pipeline(&self.path_pipeline);
            pass.set_bind_group(0, &self.overlap_bind_group.bind_group, &[]);
            if let Some((vertices, instances)) = buffers.vector_buffers {
                for instance in instances {
                    if let Ok(info) = vertices.draw_info(&instance.key) {
                        pass.set_bind_group(1, info.windfoil, &[]);
                        pass.set_vertex_buffer(0, instance.to_wgpu_buffer().slice(..));
                        pass.draw(0..4, 0..instance.len() as u32);
                    }
                }
            }
            if let Some((vertices, instances)) = buffers.glyph_buffers {
                for instance in instances {
                    if let Ok(info) = vertices.draw_info(&instance.c, &instance.direction) {
                        pass.set_bind_group(1, info.windfoil, &[]);
                        pass.set_vertex_buffer(0, instance.to_wgpu_buffer().slice(..));
                        pass.draw(0..4, 0..instance.len() as u32);
                    }
                }
            }
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Windfoil straight-alpha resolve"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
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
            multiview_mask: None,
        });
        pass.set_pipeline(&self.resolve_pipeline);
        pass.set_bind_group(0, &self.resolve_bind_group, &[]);
        pass.set_vertex_buffer(0, self.resolve_vertices.vertex_buffer.slice(..));
        pass.set_index_buffer(
            self.resolve_vertices.index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );
        pass.draw_indexed(self.resolve_vertices.index_range.clone(), 0, 0..1);
    }
}
