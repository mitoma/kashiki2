use std::{fs, sync::Mutex};

use crate::{
    debug_mode::DEBUG_FLAGS,
    overlap_bind_group::OverlapBindGroup,
    projected_curves::{self, ProjectionCache},
    rasterizer_pipeline::Buffers,
    screen_bind_group::ScreenBindGroup,
    screen_texture::ScreenTexture,
    screen_vertex_buffer::ScreenVertexBuffer,
    vector_instances::InstanceRaw,
    windfoil,
};

use crate::rasterizer_renderrer::OutlineFillRule;

pub struct RasterizerRenderrer {
    preproject_curves: bool,
    device: wgpu::Device,
    projection_pipeline: wgpu::ComputePipeline,
    projection_cache: Mutex<ProjectionCache>,
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
        Self::new_with_curve_projection(
            device,
            width,
            height,
            target_texture_format,
            enable_antialiasing,
            outline_fill_rule,
            true,
        )
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
        let path_shader = if DEBUG_FLAGS.debug_shader {
            fs::read_to_string("font_rasterizer/src/shader/windfoil.wgsl").unwrap()
        } else {
            include_str!("shader/windfoil.wgsl").to_owned()
        };
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Windfoil"),
            source: wgpu::ShaderSource::Wgsl(path_shader.as_str().into()),
        });
        let overlap_bind_group = OverlapBindGroup::new(device, width, height);
        let paths_layout = windfoil::layout(device);
        let projected_layout = projected_curves::layout(device, false);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Windfoil pipeline layout"),
            bind_group_layouts: &[
                Some(&overlap_bind_group.layout),
                Some(&paths_layout),
                Some(&projected_layout),
            ],
            immediate_size: 0,
        });
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Windfoil curve projection"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    path_shader.replace(
                        "var<storage, read> projected_points",
                        "var<storage, read_write> projected_points"
                    ),
                    projected_curves::COMPUTE_SOURCE
                )
                .into(),
            ),
        });
        let compute_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Curve projection pipeline layout"),
            bind_group_layouts: &[
                Some(&overlap_bind_group.layout),
                Some(&paths_layout),
                Some(&projected_curves::layout(device, true)),
            ],
            immediate_size: 0,
        });
        let projection_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Project glyph control points"),
                layout: Some(&compute_layout),
                module: &compute_shader,
                entry_point: Some("project_curves"),
                compilation_options: Default::default(),
                cache: None,
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
            preproject_curves,
            device: device.clone(),
            projection_pipeline,
            projection_cache: Mutex::new(ProjectionCache::default()),
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
        let mut draws = Vec::new();
        if let Some((vertices, instances)) = buffers.vector_buffers {
            for instance in instances {
                if let Some((paths, curve_count)) =
                    vertices.windfoil_draw_info(&self.device, &instance.key)
                {
                    draws.push((
                        paths,
                        curve_count,
                        instance.to_wgpu_buffer(),
                        instance.len() as u32,
                        instance.needs_curve_projection(),
                    ));
                }
            }
        }
        if let Some((vertices, instances)) = buffers.glyph_buffers {
            for instance in instances {
                if let Some((paths, curve_count)) =
                    vertices.windfoil_draw_info(&self.device, &instance.c, &instance.direction)
                {
                    draws.push((
                        paths,
                        curve_count,
                        instance.to_wgpu_buffer(),
                        instance.len() as u32,
                        instance.needs_curve_projection(),
                    ));
                }
            }
        }
        let mut cache = self.projection_cache.lock().unwrap();
        cache.begin_frame();
        let projections: Vec<_> = draws
            .iter()
            .map(|(_, curves, source, count, requested)| {
                cache.get(
                    &self.device,
                    source,
                    *curves,
                    *count,
                    *requested && self.preproject_curves,
                )
            })
            .collect();
        cache.end_frame();
        drop(cache);
        for ((paths, curves, _, count, _), projection) in draws.iter().zip(&projections) {
            if !projection.enabled || *count == 0 {
                continue;
            }
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Glyph curve projection"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.projection_pipeline);
            pass.set_bind_group(0, &self.overlap_bind_group.bind_group, &[]);
            pass.set_bind_group(1, *paths, &[]);
            pass.set_bind_group(2, &projection.compute, &[]);
            pass.dispatch_workgroups((*curves * 3).div_ceil(64), *count, 1);
        }
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
            for ((paths, _, source, count, _), projection) in draws.iter().zip(&projections) {
                pass.set_bind_group(1, *paths, &[]);
                pass.set_bind_group(2, &projection.render, &[]);
                pass.set_vertex_buffer(0, source.slice(..));
                pass.draw(0..4, 0..*count);
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
