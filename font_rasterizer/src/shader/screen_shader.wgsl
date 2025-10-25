// Vertex shader

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0)@binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}

// 普通の正規分布
/*
const WEIGHT_CENTER = 0.25;
const WEIGHT_HORIZONTAL = 0.125;
const WEIGHT_VERTICAL = 0.125;
const WEIGHT_DIAGONAL = 0.0625;
 */
// 調整されたもの
const WEIGHT_CENTER = 0.125;
const WEIGHT_HORIZONTAL = 0.0625;
const WEIGHT_VERTICAL = 0.0625;
const WEIGHT_DIAGONAL = 0.03125;
@fragment

// モーダル時に背景に回った文書にたいするボカシ処理
fn fs_main_modal_background(in: VertexOutput) -> @location(0) vec4<f32> {
    let texture_size = textureDimensions(t_diffuse);
    let texel_size = 1.0 / vec2<f32>(texture_size);

    var color = vec4<f32>(0.0);
    
    // シンプルなボックスブラー（3x3カーネル）
    let blur_radius = 5.0;
    
    // 中心のピクセル
    color += textureSample(t_diffuse, s_diffuse, in.tex_coords) * WEIGHT_CENTER;

    // 上下左右の4方向
    color += textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(blur_radius * texel_size.x, 0.0)) * WEIGHT_HORIZONTAL;
    color += textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(-blur_radius * texel_size.x, 0.0)) * WEIGHT_HORIZONTAL;
    color += textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(0.0, blur_radius * texel_size.y)) * WEIGHT_VERTICAL;
    color += textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(0.0, -blur_radius * texel_size.y)) * WEIGHT_VERTICAL;

    // 対角線の4方向
    color += textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(blur_radius * texel_size.x, blur_radius * texel_size.y)) * WEIGHT_DIAGONAL;
    color += textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(-blur_radius * texel_size.x, blur_radius * texel_size.y)) * WEIGHT_DIAGONAL;
    color += textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(blur_radius * texel_size.x, -blur_radius * texel_size.y)) * WEIGHT_DIAGONAL;
    color += textureSample(t_diffuse, s_diffuse, in.tex_coords + vec2<f32>(-blur_radius * texel_size.x, -blur_radius * texel_size.y)) * WEIGHT_DIAGONAL;

    return color;
}