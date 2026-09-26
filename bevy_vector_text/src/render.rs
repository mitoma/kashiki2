use bevy::{
    mesh::VertexBufferLayout,
    prelude::{Entity, Query, Res, ResMut, Resource, Vec4},
    render::{
        Extract,
        render_resource::{
            BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
            BindingType, BlendState, Buffer, BufferInitDescriptor, BufferUsages,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState, MultisampleState,
            PipelineCache, PrimitiveState, RenderPipelineDescriptor, Sampler, SamplerBindingType,
            SamplerDescriptor, ShaderStages, TextureDescriptor, TextureDimension, TextureFormat,
            TextureSampleType, TextureUsages, TextureViewDimension, VertexAttribute, VertexFormat,
            VertexState,
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        sync_world::RenderEntity,
        view::{Msaa, ViewTarget},
    },
};
use bytemuck::{Pod, Zeroable, cast_slice};
use std::collections::HashMap;

use crate::{VectorText, VectorTextFillRule, VectorTextGeometry};

pub(crate) const VECTOR_TEXT_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) vertex_type: u32,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) wait: vec3<f32>,
    @location(2) triangle_type: vec3<f32>,
};

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.color = input.color;
    if input.vertex_type == 0u {
        output.wait = vec3<f32>(1.0, 0.0, 0.0);
        output.triangle_type = vec3<f32>(0.0, 1.0, 0.0);
    } else if input.vertex_type == 7u {
        output.wait = vec3<f32>(0.0, 1.0, 0.0);
        output.triangle_type = vec3<f32>(0.0, 1.0, 0.0);
    } else if input.vertex_type == 8u {
        output.wait = vec3<f32>(0.0, 0.0, 1.0);
        output.triangle_type = vec3<f32>(0.0, 1.0, 0.0);
    } else if input.vertex_type == 1u {
        output.wait = vec3<f32>(1.0, 0.0, 0.0);
        output.triangle_type = vec3<f32>(0.0, 0.0, 1.0);
    } else if input.vertex_type == 3u {
        output.wait = vec3<f32>(0.0, 1.0, 0.0);
        output.triangle_type = vec3<f32>(0.0, 0.0, 1.0);
    } else if input.vertex_type == 5u {
        output.wait = vec3<f32>(0.0, 0.0, 1.0);
        output.triangle_type = vec3<f32>(0.0, 0.0, 1.0);
    } else if input.vertex_type == 2u {
        output.wait = vec3<f32>(0.0, 1.0, 0.0);
        output.triangle_type = vec3<f32>(1.0, 0.0, 0.0);
    } else if input.vertex_type == 4u {
        output.wait = vec3<f32>(0.0, 0.0, 1.0);
        output.triangle_type = vec3<f32>(1.0, 0.0, 0.0);
    } else {
        output.wait = vec3<f32>(1.0, 0.0, 0.0);
        output.triangle_type = vec3<f32>(1.0, 0.0, 0.0);
    }
    return output;
}

struct OverlapOutput {
    @location(0) color: vec4<f32>,
    @location(1) count: vec4<f32>,
};

@fragment
fn fragment(@builtin(front_facing) front_facing: bool, input: VertexOutput) -> OverlapOutput {
    let is_bezier = input.triangle_type.x > 0.5;
    let is_bezier_line = input.triangle_type.y > 0.5;
    let is_line = input.triangle_type.z > 0.5;
    let bezier_distance = pow(input.wait.x * 0.5 + input.wait.y, 2.0) - input.wait.y;
    let bezier_width = max(fwidth(bezier_distance), 0.0001);
    let bezier_alpha = 1.0 - clamp(abs(bezier_distance) / bezier_width, 0.0, 1.0);
    let line_width = max(fwidth(input.wait.x), 0.0001);
    let line_alpha = 1.0 - clamp(abs(input.wait.x) / line_width, 0.0, 1.0);
    let in_range = all(input.wait >= vec3<f32>(0.0)) && all(input.wait <= vec3<f32>(1.0));
    let winding_sign = select(-1.0, 1.0, front_facing);
    var output: OverlapOutput;
    output.color = vec4<f32>(input.color.rgb, input.color.a);
    output.count = vec4<f32>(0.0);
    if is_bezier && in_range {
        if bezier_distance < 0.0 {
            output.count.r = winding_sign;
        }
        if bezier_alpha > 0.001 && bezier_alpha < 0.999 {
            output.count.g = bezier_alpha * winding_sign;
            output.count.b = 1.0;
        }
    } else if is_bezier_line && in_range {
        output.count.r = winding_sign;
    } else if is_line && in_range {
        output.count.r = winding_sign;
        if line_alpha > 0.001 && line_alpha < 0.999 {
            output.count.g = line_alpha * winding_sign;
            output.count.b = 1.0;
        }
    }
    return output;
}

@group(0) @binding(0)
var source_texture: texture_2d<f32>;
@group(0) @binding(1)
var source_sampler: sampler;
@group(0) @binding(2)
var count_texture: texture_2d<f32>;

struct ResolveOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn resolve_vertex(@builtin(vertex_index) index: u32) -> ResolveOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 2.0),
        vec2<f32>(2.0, 0.0),
        vec2<f32>(0.0, 0.0),
    );
    var output: ResolveOutput;
    output.position = vec4<f32>(positions[index], 0.0, 1.0);
    output.uv = uvs[index];
    return output;
}

fn resolve_fragment_impl(uv_input: vec2<f32>, even_odd: bool) -> vec4<f32> {
    let source = textureSample(source_texture, source_sampler, uv_input);
    let uv = clamp(uv_input, vec2<f32>(0.0), vec2<f32>(0.999999));
    let size = vec2<f32>(textureDimensions(count_texture));
    let counts = textureLoad(count_texture, vec2<i32>(uv * size), 0);
    let non_zero_coverage = select(
        clamp(abs(counts.g) / max(counts.b, 1.0), 0.0, 1.0),
        clamp(abs(counts.r), 0.0, 1.0),
        counts.b == 0.0,
    );
    let crossings = abs(counts.r);
    let even_odd_coverage = crossings - 2.0 * floor(crossings * 0.5);
    let coverage = select(non_zero_coverage, even_odd_coverage, even_odd);
    return vec4<f32>(source.rgb, min(source.a, clamp(coverage, 0.0, 1.0)));
}

@fragment
fn resolve_fragment_even_odd(input: ResolveOutput) -> @location(0) vec4<f32> {
    return resolve_fragment_impl(input.uv, true);
}

@fragment
fn resolve_fragment_non_zero(input: ResolveOutput) -> @location(0) vec4<f32> {
    return resolve_fragment_impl(input.uv, false);
}
"#;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ExtractedVectorText {
    pub(crate) entity: Entity,
    pub(crate) text: String,
    pub(crate) font_size: f32,
    pub(crate) color: Vec4,
}

#[derive(Default, Resource)]
pub(crate) struct ExtractedVectorTexts {
    pub(crate) values: Vec<ExtractedVectorText>,
}

#[derive(Default, Resource)]
pub(crate) struct PreparedVectorTexts {
    pub(crate) values: Vec<ExtractedVectorText>,
}

#[derive(Clone, Debug)]
pub(crate) struct ExtractedVectorGeometry {
    pub(crate) entity: Entity,
    pub(crate) data: font_rasterizer::VectorVertexData,
    pub(crate) color: Vec4,
}

#[derive(Default, Resource)]
pub(crate) struct ExtractedVectorGeometries {
    pub(crate) values: Vec<ExtractedVectorGeometry>,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuVectorVertex {
    position: [f32; 2],
    vertex_type: u32,
    color: [f32; 4],
}

#[allow(dead_code)]
struct GpuVectorTextBuffer {
    vertex: Buffer,
    index: Buffer,
    index_count: u32,
}

#[derive(Default, Resource)]
pub(crate) struct GpuVectorTextBuffers {
    values: HashMap<Entity, GpuVectorTextBuffer>,
}

#[derive(Resource)]
pub(crate) struct VectorTextShader(pub(crate) bevy::prelude::Handle<bevy::shader::Shader>);

#[derive(Resource)]
pub(crate) struct VectorTextPipeline {
    pub(crate) pipeline_ids: HashMap<u32, CachedRenderPipelineId>,
    pub(crate) resolve_pipeline_ids: HashMap<(u32, bool), CachedRenderPipelineId>,
    pub(crate) resolve_layout: BindGroupLayout,
    pub(crate) resolve_sampler: Sampler,
}

pub(crate) fn extract_vector_texts(
    mut extracted: ResMut<ExtractedVectorTexts>,
    query: Extract<Query<(RenderEntity, &VectorText)>>,
) {
    extracted.values.clear();
    extracted
        .values
        .extend(query.iter().map(|(entity, text)| ExtractedVectorText {
            entity,
            text: text.text.clone(),
            font_size: text.font_size,
            color: text.color,
        }));
}

pub(crate) fn prepare_vector_texts(
    extracted: bevy::ecs::system::Res<ExtractedVectorTexts>,
    mut prepared: ResMut<PreparedVectorTexts>,
) {
    prepared.values.clear();
    prepared.values.extend(extracted.values.iter().cloned());
}

pub(crate) fn extract_vector_geometries(
    mut extracted: ResMut<ExtractedVectorGeometries>,
    query: Extract<Query<(RenderEntity, &VectorTextGeometry, &VectorText)>>,
) {
    extracted.values.clear();
    extracted.values.extend(
        query
            .iter()
            .map(|(entity, geometry, text)| ExtractedVectorGeometry {
                entity,
                data: geometry.data.clone(),
                color: text.color,
            }),
    );
}

pub(crate) fn prepare_vector_text_buffers(
    render_device: Res<RenderDevice>,
    geometries: Res<ExtractedVectorGeometries>,
    mut buffers: ResMut<GpuVectorTextBuffers>,
) {
    buffers.values.clear();
    for geometry in &geometries.values {
        let vertices = geometry
            .data
            .positions
            .iter()
            .copied()
            .zip(geometry.data.vertex_types.iter().copied())
            .map(|(position, vertex_type)| GpuVectorVertex {
                position,
                vertex_type,
                color: geometry.color.to_array(),
            })
            .collect::<Vec<_>>();
        let vertex = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Bevy Vector Text Vertex Buffer"),
            contents: cast_slice(&vertices),
            usage: BufferUsages::VERTEX,
        });
        let index = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Bevy Vector Text Index Buffer"),
            contents: cast_slice(&geometry.data.indices),
            usage: BufferUsages::INDEX,
        });
        buffers.values.insert(
            geometry.entity,
            GpuVectorTextBuffer {
                vertex,
                index,
                index_count: geometry.data.indices.len() as u32,
            },
        );
    }
}

pub(crate) fn init_vector_text_pipeline(
    mut commands: bevy::ecs::system::Commands,
    shader: Res<VectorTextShader>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
) {
    let pipeline_ids = [1, 2, 4, 8]
        .into_iter()
        .map(|sample_count| {
            let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some(format!("Bevy Vector Text Pipeline {sample_count}x MSAA").into()),
                layout: vec![],
                vertex: VertexState {
                    shader: shader.0.clone(),
                    entry_point: Some("vertex".into()),
                    buffers: vec![VertexBufferLayout {
                        array_stride: std::mem::size_of::<GpuVectorVertex>() as u64,
                        step_mode: bevy::render::render_resource::VertexStepMode::Vertex,
                        attributes: vec![
                            VertexAttribute {
                                format: VertexFormat::Float32x2,
                                offset: 0,
                                shader_location: 0,
                            },
                            VertexAttribute {
                                format: VertexFormat::Uint32,
                                offset: std::mem::size_of::<[f32; 2]>() as u64,
                                shader_location: 1,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32x4,
                                offset: std::mem::size_of::<[f32; 2]>() as u64
                                    + std::mem::size_of::<u32>() as u64,
                                shader_location: 2,
                            },
                        ],
                    }],
                    ..Default::default()
                },
                fragment: Some(FragmentState {
                    shader: shader.0.clone(),
                    shader_defs: vec![],
                    entry_point: Some("fragment".into()),
                    targets: vec![
                        Some(ColorTargetState {
                            format: TextureFormat::Rgba8UnormSrgb,
                            blend: Some(BlendState::REPLACE),
                            write_mask: ColorWrites::ALL,
                        }),
                        Some(ColorTargetState {
                            format: TextureFormat::Rgba16Float,
                            blend: Some(BlendState {
                                color: bevy::render::render_resource::BlendComponent {
                                    src_factor: bevy::render::render_resource::BlendFactor::One,
                                    dst_factor: bevy::render::render_resource::BlendFactor::One,
                                    operation: bevy::render::render_resource::BlendOperation::Add,
                                },
                                alpha: bevy::render::render_resource::BlendComponent {
                                    src_factor: bevy::render::render_resource::BlendFactor::One,
                                    dst_factor: bevy::render::render_resource::BlendFactor::One,
                                    operation: bevy::render::render_resource::BlendOperation::Add,
                                },
                            }),
                            write_mask: ColorWrites::ALL,
                        }),
                    ],
                }),
                primitive: PrimitiveState::default(),
                multisample: MultisampleState {
                    count: sample_count,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                ..Default::default()
            });
            (sample_count, pipeline_id)
        })
        .collect();
    let resolve_layout_descriptor = BindGroupLayoutDescriptor::new(
        "Bevy Vector Text Resolve Bind Group Layout",
        &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
        ],
    );
    let resolve_layout = render_device.create_bind_group_layout(
        "Bevy Vector Text Resolve Bind Group Layout",
        &resolve_layout_descriptor.entries,
    );
    let resolve_sampler = render_device.create_sampler(&SamplerDescriptor::default());
    let mut resolve_pipeline_ids = HashMap::new();
    for sample_count in [1, 2, 4, 8] {
        for even_odd in [false, true] {
            let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some(
                    format!(
                        "Bevy Vector Text Resolve Pipeline {sample_count}x MSAA {}",
                        if even_odd { "EvenOdd" } else { "NonZero" }
                    )
                    .into(),
                ),
                layout: vec![resolve_layout_descriptor.clone()],
                vertex: VertexState {
                    shader: shader.0.clone(),
                    entry_point: Some("resolve_vertex".into()),
                    buffers: vec![],
                    ..Default::default()
                },
                fragment: Some(FragmentState {
                    shader: shader.0.clone(),
                    shader_defs: vec![],
                    entry_point: Some(
                        if even_odd {
                            "resolve_fragment_even_odd"
                        } else {
                            "resolve_fragment_non_zero"
                        }
                        .into(),
                    ),
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::Rgba8UnormSrgb,
                        blend: Some(BlendState::ALPHA_BLENDING),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState::default(),
                multisample: MultisampleState {
                    count: sample_count,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                ..Default::default()
            });
            resolve_pipeline_ids.insert((sample_count, even_odd), pipeline_id);
        }
    }
    commands.insert_resource(VectorTextPipeline {
        pipeline_ids,
        resolve_pipeline_ids,
        resolve_layout,
        resolve_sampler,
    });
}

pub(crate) fn draw_vector_texts(
    view: ViewQuery<(&ViewTarget, &Msaa)>,
    pipeline: Option<Res<VectorTextPipeline>>,
    fill_rule: Res<VectorTextFillRule>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
    buffers: Res<GpuVectorTextBuffers>,
    mut context: RenderContext,
) {
    let Some(pipeline) = pipeline else {
        return;
    };
    let (target, msaa) = view.into_inner();
    let Some(pipeline_id) = pipeline.pipeline_ids.get(&1) else {
        return;
    };
    let Some(glyph_pipeline) = pipeline_cache.get_render_pipeline(*pipeline_id) else {
        return;
    };
    let even_odd = *fill_rule == VectorTextFillRule::EvenOdd;
    let Some(resolve_pipeline_id) = pipeline
        .resolve_pipeline_ids
        .get(&(msaa.samples(), even_odd))
    else {
        return;
    };
    let Some(resolve_pipeline) = pipeline_cache.get_render_pipeline(*resolve_pipeline_id) else {
        return;
    };
    if buffers.values.is_empty() {
        return;
    }

    let extent = target.main_texture().size();
    let intermediate = render_device.create_texture(&TextureDescriptor {
        label: Some("Bevy Vector Text Intermediate Texture"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let intermediate_view = intermediate.create_view(&Default::default());
    let count_texture = render_device.create_texture(&TextureDescriptor {
        label: Some("Bevy Vector Text Count Texture"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let count_view = count_texture.create_view(&Default::default());

    {
        let mut render_pass = context.begin_tracked_render_pass(
            bevy::render::render_resource::RenderPassDescriptor {
                label: Some("Bevy Vector Text Overlap Pass"),
                color_attachments: &[
                    Some(bevy::render::render_resource::RenderPassColorAttachment {
                        view: &intermediate_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: bevy::render::render_resource::Operations {
                            load: bevy::render::render_resource::LoadOp::Clear(Default::default()),
                            store: bevy::render::render_resource::StoreOp::Store,
                        },
                    }),
                    Some(bevy::render::render_resource::RenderPassColorAttachment {
                        view: &count_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: bevy::render::render_resource::Operations {
                            load: bevy::render::render_resource::LoadOp::Clear(Default::default()),
                            store: bevy::render::render_resource::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            },
        );
        render_pass.set_render_pipeline(glyph_pipeline);
        for buffer in buffers.values.values() {
            render_pass.set_vertex_buffer(0, buffer.vertex.slice(..));
            render_pass.set_index_buffer(
                buffer.index.slice(..),
                bevy::render::render_resource::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..buffer.index_count, 0, 0..1);
        }
    }

    let bind_group = render_device.create_bind_group(
        "Bevy Vector Text Resolve Bind Group",
        &pipeline.resolve_layout,
        &[
            bevy::render::render_resource::BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&intermediate_view),
            },
            bevy::render::render_resource::BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(&pipeline.resolve_sampler),
            },
            bevy::render::render_resource::BindGroupEntry {
                binding: 2,
                resource: BindingResource::TextureView(&count_view),
            },
        ],
    );
    let mut resolve_pass =
        context.begin_tracked_render_pass(bevy::render::render_resource::RenderPassDescriptor {
            label: Some("Bevy Vector Text Outline Resolve Pass"),
            color_attachments: &[Some(target.get_color_attachment())],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    resolve_pass.set_render_pipeline(resolve_pipeline);
    resolve_pass.set_bind_group(0, &bind_group, &[]);
    resolve_pass.draw(0..3, 0..1);
}
