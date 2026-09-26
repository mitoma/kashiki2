use bevy::{
    mesh::VertexBufferLayout,
    prelude::{Entity, Query, Res, ResMut, Resource, Vec4},
    render::{
        Extract,
        render_resource::{
            BindGroup, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
            BindingResource, BindingType, BlendState, Buffer, BufferBindingType,
            BufferInitDescriptor, BufferUsages, CachedRenderPipelineId, ColorTargetState,
            ColorWrites, FragmentState, MultisampleState, PipelineCache, PrimitiveState,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
            TextureUsages, TextureView, TextureViewDimension, VertexAttribute, VertexFormat,
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

fn linerstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    return clamp((value - edge0) / (edge1 - edge0), 0.0, 1.0);
}

@fragment
fn fragment(@builtin(front_facing) front_facing: bool, input: VertexOutput) -> OverlapOutput {
    let is_bezier = input.triangle_type.x > 0.5;
    let is_bezier_line = input.triangle_type.y > 0.5;
    let is_line = input.triangle_type.z > 0.5;
    let bezier_distance = pow(input.wait.x * 0.5 + input.wait.y, 2.0) - input.wait.y;
    let bezier_width = max(fwidth(bezier_distance), 0.0001);
    let bezier_alpha = 1.0 - linerstep(
        -bezier_width / 2.0,
        bezier_width / 2.0,
        abs(bezier_distance),
    );
    let line_width = max(fwidth(input.wait.x), 0.0001);
    let line_alpha = 1.0 - linerstep(-line_width / 2.0, line_width / 2.0, abs(input.wait.x));
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

const IDENTITY_MATRIX: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

pub(crate) fn bevy_adapter_shader(_canonical_overlap_shader: &str) -> String {
    VECTOR_TEXT_SHADER.to_owned()
}

pub(crate) fn bevy_overlap_shader(canonical_shader: &str) -> String {
    canonical_shader
        .replace(
            "    @location(1) vertex_type: u32,",
            "    @location(1) vertex_type: u32,\n    @location(2) color: vec4<f32>,",
        )
        .replace(
            "    @location(2) triangle_type: vec3<f32>,",
            "    @location(2) triangle_type: vec3<f32>,\n    @location(3) alpha: f32,",
        )
        .replace(
            "    out.color = instances.color;",
            "    out.color = instances.color;\n    out.alpha = model.color.a;",
        )
        .replace(
            "    output.color = vec4<f32>(in.color.rgb, 0f);",
            "    output.color = vec4<f32>(in.color.rgb, in.alpha);",
        )
}

pub(crate) fn bevy_outline_shader(canonical_shader: &str) -> String {
    canonical_shader
        .replace(
            "return vec4<f32>(color.rgb, 1.0 - alpha);",
            "return vec4<f32>(color.rgb, (1.0 - alpha) * color.a);",
        )
        .replace(
            "return vec4<f32>(color.rgb, alpha);",
            "return vec4<f32>(color.rgb, alpha * color.a);",
        )
        .replace(
            "return vec4<f32>(color.rgb, 1.0);",
            "return vec4<f32>(color.rgb, color.a);",
        )
}

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

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuOutlineUniforms {
    width: u32,
    padding: [u32; 3],
}

#[allow(dead_code)]
struct GpuVectorTextBuffer {
    vertex: Buffer,
    instance: Buffer,
    index: Buffer,
    index_count: u32,
}

#[derive(Default, Resource)]
pub(crate) struct GpuVectorTextBuffers {
    values: HashMap<Entity, GpuVectorTextBuffer>,
}

#[derive(Resource)]
pub(crate) struct VectorTextFullscreenShader(
    pub(crate) bevy::prelude::Handle<bevy::shader::Shader>,
);

#[derive(Resource)]
pub(crate) struct VectorTextOverlapShader(pub(crate) bevy::prelude::Handle<bevy::shader::Shader>);

#[derive(Resource)]
pub(crate) struct VectorTextOutlineShader(pub(crate) bevy::prelude::Handle<bevy::shader::Shader>);

#[derive(Resource)]
pub(crate) struct VectorTextPipeline {
    pub(crate) overlap_pipeline_ids: HashMap<bool, CachedRenderPipelineId>,
    pub(crate) outline_pipeline_ids: HashMap<(u32, bool), CachedRenderPipelineId>,
    pub(crate) overlap_bind_group: bevy::render::render_resource::BindGroup,
    pub(crate) outline_layout: BindGroupLayout,
    pub(crate) outline_sampler: Sampler,
    views: HashMap<Entity, CachedVectorTextView>,
}

struct CachedVectorTextView {
    width: u32,
    height: u32,
    depth_or_array_layers: u32,
    sample_count: u32,
    intermediate_format: TextureFormat,
    count_format: TextureFormat,
    _intermediate_texture: Texture,
    intermediate_view: TextureView,
    _count_texture: Texture,
    count_view: TextureView,
    outline_bind_group: BindGroup,
}

impl CachedVectorTextView {
    fn matches(&self, extent: bevy::render::render_resource::Extent3d, sample_count: u32) -> bool {
        self.width == extent.width
            && self.height == extent.height
            && self.depth_or_array_layers == extent.depth_or_array_layers
            && self.sample_count == sample_count
            && self.intermediate_format == TextureFormat::Rgba8UnormSrgb
            && self.count_format == TextureFormat::Rgba16Float
    }
}

fn create_cached_vector_text_view(
    render_device: &RenderDevice,
    extent: bevy::render::render_resource::Extent3d,
    sample_count: u32,
    outline_layout: &BindGroupLayout,
    outline_sampler: &Sampler,
) -> CachedVectorTextView {
    let intermediate_format = TextureFormat::Rgba8UnormSrgb;
    let count_format = TextureFormat::Rgba16Float;
    let intermediate_texture = render_device.create_texture(&TextureDescriptor {
        label: Some("Bevy Vector Text Intermediate Texture"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: intermediate_format,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let intermediate_view = intermediate_texture.create_view(&Default::default());
    let count_texture = render_device.create_texture(&TextureDescriptor {
        label: Some("Bevy Vector Text Count Texture"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: count_format,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let count_view = count_texture.create_view(&Default::default());
    let outline_uniforms = GpuOutlineUniforms {
        width: extent.width,
        padding: [0; 3],
    };
    let outline_uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Bevy Vector Text Outline Uniform Buffer"),
        contents: cast_slice(&[outline_uniforms]),
        usage: BufferUsages::UNIFORM,
    });
    let outline_bind_group = render_device.create_bind_group(
        "Bevy Vector Text Canonical Outline Bind Group",
        outline_layout,
        &[
            bevy::render::render_resource::BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&intermediate_view),
            },
            bevy::render::render_resource::BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(outline_sampler),
            },
            bevy::render::render_resource::BindGroupEntry {
                binding: 2,
                resource: outline_uniform_buffer.as_entire_binding(),
            },
            bevy::render::render_resource::BindGroupEntry {
                binding: 3,
                resource: BindingResource::TextureView(&count_view),
            },
        ],
    );

    CachedVectorTextView {
        width: extent.width,
        height: extent.height,
        depth_or_array_layers: extent.depth_or_array_layers,
        sample_count,
        intermediate_format,
        count_format,
        _intermediate_texture: intermediate_texture,
        intermediate_view,
        _count_texture: count_texture,
        count_view,
        outline_bind_group,
    }
}

fn vector_text_vertex_buffer_layouts() -> Vec<VertexBufferLayout> {
    vec![
        VertexBufferLayout {
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
        },
        VertexBufferLayout {
            array_stride: std::mem::size_of::<font_rasterizer::shader_contract::InstanceRaw>()
                as u64,
            step_mode: bevy::render::render_resource::VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 5,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 6,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 32,
                    shader_location: 7,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 48,
                    shader_location: 8,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 64,
                    shader_location: 9,
                },
                VertexAttribute {
                    format: VertexFormat::Uint32,
                    offset: 76,
                    shader_location: 10,
                },
                VertexAttribute {
                    format: VertexFormat::Uint32,
                    offset: 80,
                    shader_location: 11,
                },
                VertexAttribute {
                    format: VertexFormat::Float32,
                    offset: 84,
                    shader_location: 12,
                },
                VertexAttribute {
                    format: VertexFormat::Uint32,
                    offset: 88,
                    shader_location: 13,
                },
            ],
        },
    ]
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
        let instance = font_rasterizer::shader_contract::InstanceRaw {
            model: IDENTITY_MATRIX,
            color: [geometry.color.x, geometry.color.y, geometry.color.z],
            motion: 0,
            start_time: 0,
            gain: 0.0,
            duration: 0,
        };
        let instance = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Bevy Vector Text Instance Buffer"),
            contents: cast_slice(&[instance]),
            usage: BufferUsages::VERTEX,
        });
        buffers.values.insert(
            geometry.entity,
            GpuVectorTextBuffer {
                vertex,
                instance,
                index,
                index_count: geometry.data.indices.len() as u32,
            },
        );
    }
}

pub(crate) fn init_vector_text_pipeline(
    mut commands: bevy::ecs::system::Commands,
    fullscreen_shader: Res<VectorTextFullscreenShader>,
    overlap_shader: Res<VectorTextOverlapShader>,
    outline_shader: Res<VectorTextOutlineShader>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
) {
    let overlap_layout_descriptor = BindGroupLayoutDescriptor::new(
        "Bevy Vector Text Overlap Bind Group Layout",
        &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    );
    let overlap_layout = render_device.create_bind_group_layout(
        "Bevy Vector Text Overlap Bind Group Layout",
        &overlap_layout_descriptor.entries,
    );
    let overlap_uniforms = font_rasterizer::shader_contract::OverlapUniforms {
        view_proj: IDENTITY_MATRIX,
        default_view_proj: IDENTITY_MATRIX,
        time: 0,
        width: 0,
        enable_antialiasing: 1,
        padding: [0],
    };
    let overlap_uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Bevy Vector Text Overlap Uniform Buffer"),
        contents: cast_slice(&[overlap_uniforms]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    let overlap_bind_group = render_device.create_bind_group(
        "Bevy Vector Text Overlap Bind Group",
        &overlap_layout,
        &[bevy::render::render_resource::BindGroupEntry {
            binding: 0,
            resource: overlap_uniform_buffer.as_entire_binding(),
        }],
    );

    let overlap_pipeline_ids = [false, true]
        .into_iter()
        .map(|even_odd| {
            let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some(
                    format!(
                        "Bevy Vector Text Canonical Overlap {}",
                        if even_odd { "EvenOdd" } else { "NonZero" }
                    )
                    .into(),
                ),
                layout: vec![overlap_layout_descriptor.clone()],
                vertex: VertexState {
                    shader: overlap_shader.0.clone(),
                    entry_point: Some("vs_main".into()),
                    buffers: vector_text_vertex_buffer_layouts(),
                    ..Default::default()
                },
                fragment: Some(FragmentState {
                    shader: overlap_shader.0.clone(),
                    shader_defs: vec![],
                    entry_point: Some(
                        if even_odd {
                            "fs_main_even_odd"
                        } else {
                            "fs_main_non_zero"
                        }
                        .into(),
                    ),
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
                multisample: MultisampleState::default(),
                ..Default::default()
            });
            (even_odd, pipeline_id)
        })
        .collect();

    let outline_layout_descriptor = BindGroupLayoutDescriptor::new(
        "Bevy Vector Text Canonical Outline Bind Group Layout",
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
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
        ],
    );
    let outline_layout = render_device.create_bind_group_layout(
        "Bevy Vector Text Canonical Outline Bind Group Layout",
        &outline_layout_descriptor.entries,
    );
    let outline_sampler = render_device.create_sampler(&SamplerDescriptor::default());
    let mut outline_pipeline_ids = HashMap::new();
    for sample_count in [1, 2, 4, 8] {
        for even_odd in [false, true] {
            let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some(
                    format!(
                        "Bevy Vector Text Canonical Outline {sample_count}x MSAA {}",
                        if even_odd { "EvenOdd" } else { "NonZero" }
                    )
                    .into(),
                ),
                layout: vec![outline_layout_descriptor.clone()],
                vertex: VertexState {
                    shader: fullscreen_shader.0.clone(),
                    entry_point: Some("resolve_vertex".into()),
                    buffers: vec![],
                    ..Default::default()
                },
                fragment: Some(FragmentState {
                    shader: outline_shader.0.clone(),
                    shader_defs: vec![],
                    entry_point: Some(
                        if even_odd {
                            "fs_main_even_odd"
                        } else {
                            "fs_main_non_zero"
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
            outline_pipeline_ids.insert((sample_count, even_odd), pipeline_id);
        }
    }
    commands.insert_resource(VectorTextPipeline {
        overlap_pipeline_ids,
        outline_pipeline_ids,
        overlap_bind_group,
        outline_layout,
        outline_sampler,
        views: HashMap::new(),
    });
}

pub(crate) fn draw_vector_texts(
    view: ViewQuery<(&ViewTarget, &Msaa)>,
    pipeline: Option<ResMut<VectorTextPipeline>>,
    fill_rule: Res<VectorTextFillRule>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
    buffers: Res<GpuVectorTextBuffers>,
    mut context: RenderContext,
) {
    let Some(mut pipeline) = pipeline else {
        return;
    };
    let view_entity = view.entity();
    let (target, msaa) = view.into_inner();
    let even_odd = *fill_rule == VectorTextFillRule::EvenOdd;
    let Some(pipeline_id) = pipeline.overlap_pipeline_ids.get(&even_odd) else {
        return;
    };
    let Some(glyph_pipeline) = pipeline_cache.get_render_pipeline(*pipeline_id) else {
        return;
    };
    let Some(outline_pipeline_id) = pipeline
        .outline_pipeline_ids
        .get(&(msaa.samples(), even_odd))
    else {
        return;
    };
    let Some(outline_pipeline) = pipeline_cache.get_render_pipeline(*outline_pipeline_id) else {
        return;
    };
    if buffers.values.is_empty() {
        return;
    }

    let extent = target.main_texture().size();
    let sample_count = msaa.samples();
    let needs_refresh = pipeline
        .views
        .get(&view_entity)
        .is_none_or(|cached| !cached.matches(extent, sample_count));
    if needs_refresh {
        let cached_view = create_cached_vector_text_view(
            &render_device,
            extent,
            sample_count,
            &pipeline.outline_layout,
            &pipeline.outline_sampler,
        );
        pipeline.views.insert(view_entity, cached_view);
    }
    let cached_view = pipeline
        .views
        .get(&view_entity)
        .expect("view resources were created or reused");

    {
        let mut render_pass = context.begin_tracked_render_pass(
            bevy::render::render_resource::RenderPassDescriptor {
                label: Some("Bevy Vector Text Overlap Pass"),
                color_attachments: &[
                    Some(bevy::render::render_resource::RenderPassColorAttachment {
                        view: &cached_view.intermediate_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: bevy::render::render_resource::Operations {
                            load: bevy::render::render_resource::LoadOp::Clear(Default::default()),
                            store: bevy::render::render_resource::StoreOp::Store,
                        },
                    }),
                    Some(bevy::render::render_resource::RenderPassColorAttachment {
                        view: &cached_view.count_view,
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
        render_pass.set_bind_group(0, &pipeline.overlap_bind_group, &[]);
        for buffer in buffers.values.values() {
            render_pass.set_vertex_buffer(0, buffer.vertex.slice(..));
            render_pass.set_vertex_buffer(1, buffer.instance.slice(..));
            render_pass.set_index_buffer(
                buffer.index.slice(..),
                bevy::render::render_resource::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..buffer.index_count, 0, 0..1);
        }
    }

    let mut outline_pass =
        context.begin_tracked_render_pass(bevy::render::render_resource::RenderPassDescriptor {
            label: Some("Bevy Vector Text Canonical Outline Pass"),
            color_attachments: &[Some(target.get_color_attachment())],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    outline_pass.set_render_pipeline(outline_pipeline);
    outline_pass.set_bind_group(0, &cached_view.outline_bind_group, &[]);
    outline_pass.draw(0..3, 0..1);
}

pub(crate) fn cleanup_vector_text_view_cache(
    mut pipeline: ResMut<VectorTextPipeline>,
    active_views: Query<Entity, bevy::ecs::query::With<ViewTarget>>,
) {
    pipeline
        .views
        .retain(|view_entity, _| active_views.get(*view_entity).is_ok());
}

#[cfg(test)]
mod tests {
    use super::{bevy_outline_shader, bevy_overlap_shader};

    #[test]
    fn canonical_shader_adapters_preserve_vector_text_alpha() {
        let overlap = bevy_overlap_shader(font_rasterizer::shader_sources::OVERLAP);
        assert!(overlap.contains("@location(2) color: vec4<f32>"));
        assert!(overlap.contains("out.alpha = model.color.a;"));
        assert!(overlap.contains("output.color = vec4<f32>(in.color.rgb, in.alpha);"));

        let outline = bevy_outline_shader(font_rasterizer::shader_sources::OUTLINE);
        assert!(outline.contains("(1.0 - alpha) * color.a"));
        assert!(outline.contains("alpha * color.a"));
        assert!(outline.contains("return vec4<f32>(color.rgb, color.a);"));
    }
}
