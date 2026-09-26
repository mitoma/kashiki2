use bevy::{
    mesh::VertexBufferLayout,
    prelude::{Entity, Query, Res, ResMut, Resource, Vec4},
    render::{
        Extract,
        render_resource::{
            BlendState, Buffer, BufferInitDescriptor, BufferUsages, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, FragmentState, MultisampleState, PipelineCache,
            PrimitiveState, RenderPipelineDescriptor, TextureFormat, VertexAttribute, VertexFormat,
            VertexState,
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        sync_world::RenderEntity,
        view::{Msaa, ViewTarget},
    },
};
use bytemuck::{Pod, Zeroable, cast_slice};
use std::collections::HashMap;

use crate::{VectorText, VectorTextGeometry};

pub(crate) const VECTOR_TEXT_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) vertex_type: u32,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
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
            (sample_count, pipeline_id)
        })
        .collect();
    commands.insert_resource(VectorTextPipeline { pipeline_ids });
}

pub(crate) fn draw_vector_texts(
    view: ViewQuery<(&ViewTarget, &Msaa)>,
    pipeline: Option<Res<VectorTextPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    buffers: Res<GpuVectorTextBuffers>,
    mut context: RenderContext,
) {
    let Some(pipeline) = pipeline else {
        return;
    };
    let (target, msaa) = view.into_inner();
    let Some(pipeline_id) = pipeline.pipeline_ids.get(&msaa.samples()) else {
        return;
    };
    let Some(pipeline) = pipeline_cache.get_render_pipeline(*pipeline_id) else {
        return;
    };
    if buffers.values.is_empty() {
        return;
    }

    let mut render_pass =
        context.begin_tracked_render_pass(bevy::render::render_resource::RenderPassDescriptor {
            label: Some("Bevy Vector Text Pass"),
            color_attachments: &[Some(target.get_color_attachment())],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    render_pass.set_render_pipeline(pipeline);
    for buffer in buffers.values.values() {
        render_pass.set_vertex_buffer(0, buffer.vertex.slice(..));
        render_pass.set_index_buffer(
            buffer.index.slice(..),
            bevy::render::render_resource::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(0..buffer.index_count, 0, 0..1);
    }
}
