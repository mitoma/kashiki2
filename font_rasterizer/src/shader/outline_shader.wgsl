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
struct Uniforms {
    u_width: u32,
    u_height: u32,
};

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0)@binding(1)
var s_diffuse: sampler;
@group(0) @binding(2)
var<uniform> u_buffer: Uniforms;
@group(0) @binding(3)
var t_overlap_count: texture_2d<f32>;

const UNIT: f32 = 0.00390625;
const WINDING_THRESHOLD: f32 = 0.001;

/// 指定した座標のピクセルが「外側」かどうかを判定する。
/// 外側の条件: winding(r) がほぼゼロで、かつ alpha_counts(b) が非ゼロ。
fn is_sample_outside(coords: vec2<f32>) -> bool {
    let sample = textureSample(t_overlap_count, s_diffuse, coords);
    return (abs(sample.r) <= WINDING_THRESHOLD) && (abs(sample.b) > WINDING_THRESHOLD);
}

/// 周辺8ピクセルのいずれかが「外側」かどうかを判定する。
fn check_around_is_any_outside(tex_coords: vec2<f32>) -> bool {
    let dx = 1.0 / f32(u_buffer.u_width);
    let dy = 1.0 / f32(u_buffer.u_height);
    var result = false;
    result = result || is_sample_outside(tex_coords + vec2<f32>(-dx, 0.0));
    result = result || is_sample_outside(tex_coords + vec2<f32>(dx, 0.0));
    result = result || is_sample_outside(tex_coords + vec2<f32>(0.0, -dy));
    result = result || is_sample_outside(tex_coords + vec2<f32>(0.0, dy));
    result = result || is_sample_outside(tex_coords + vec2<f32>(-dx, -dy));
    result = result || is_sample_outside(tex_coords + vec2<f32>(dx, -dy));
    result = result || is_sample_outside(tex_coords + vec2<f32>(-dx, dy));
    result = result || is_sample_outside(tex_coords + vec2<f32>(dx, dy));
    return result;
}

/// 指定した座標のピクセルが「内側」かどうかを判定する。
/// 内側の条件: winding(r) が非ゼロで、かつ alpha_counts(b) が非ゼロ。
fn is_sample_inside(coords: vec2<f32>) -> bool {
    let sample = textureSample(t_overlap_count, s_diffuse, coords);
    return (abs(sample.r) > WINDING_THRESHOLD) && (abs(sample.b) > WINDING_THRESHOLD);
}

/// 周辺8ピクセルのいずれかが「内側」かどうかを判定する。
fn check_around_is_any_inside(tex_coords: vec2<f32>) -> bool {
    let dx = 1.0 / f32(u_buffer.u_width);
    let dy = 1.0 / f32(u_buffer.u_height);
    var result = false;
    result = result || is_sample_inside(tex_coords + vec2<f32>(-dx, 0.0));
    result = result || is_sample_inside(tex_coords + vec2<f32>(dx, 0.0));
    result = result || is_sample_inside(tex_coords + vec2<f32>(0.0, -dy));
    result = result || is_sample_inside(tex_coords + vec2<f32>(0.0, dy));
    result = result || is_sample_inside(tex_coords + vec2<f32>(-dx, -dy));
    result = result || is_sample_inside(tex_coords + vec2<f32>(dx, -dy));
    result = result || is_sample_inside(tex_coords + vec2<f32>(-dx, dy));
    result = result || is_sample_inside(tex_coords + vec2<f32>(dx, dy));
    return result;
}

@fragment
fn fs_main_even_odd(in: VertexOutput) -> @location(0) vec4<f32> {
    // テクスチャから色を取得
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    // 重なり回数テクスチャから値を取得（Rgba16Float: 符号付き浮動小数点）
    let overlap_count = textureSample(t_overlap_count, s_diffuse, in.tex_coords);

    let counts = u32(abs(overlap_count.r) / UNIT);
    let alpha_accum = overlap_count.g;
    let alpha_counts = u32(abs(overlap_count.b) / UNIT);
    var alpha = 0.0;
    if alpha_counts != 0u {
        alpha = clamp(abs(alpha_accum) / f32(alpha_counts), 0.0, 1.0);
    }

    // EvenOdd Rule
    if counts % 2u == 1u {
        if alpha_counts % 2u == 1u {
            return vec4<f32>(color.rgb, alpha);
        } else {
            return vec4<f32>(color.rgb, 1f - alpha);
        }
    } else {
        if alpha_counts % 2u == 1u {
            return vec4<f32>(color.rgb, 1f - alpha);
        } else {
            return vec4<f32>(color.rgb, alpha);
        }
    }
}

@fragment
fn fs_main_non_zero(in: VertexOutput) -> @location(0) vec4<f32> {
    // テクスチャから色を取得
    var color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    // 重なり回数テクスチャから値を取得（Rgba16Float: 符号付き浮動小数点）
    let overlap_count = textureSample(t_overlap_count, s_diffuse, in.tex_coords);

    let winding = overlap_count.r;
    let alpha_accum = overlap_count.g;
    let alpha_counts = overlap_count.b;

    var edge_overlapped = false;
    if overlap_count.a / UNIT >= 2.0 {
        edge_overlapped = true;
    }

    // Non-Zero Winding Rule: winding が非ゼロなら内側
    let is_inside = abs(winding) > WINDING_THRESHOLD;

    // 周辺8ピクセルのいずれかが外側かどうかを取得
    let around_is_any_outside = check_around_is_any_outside(in.tex_coords);
    // 周辺8ピクセルのいずれかが内側かどうかを取得
    let around_is_any_inside = check_around_is_any_inside(in.tex_coords);

    // 周辺8ピクセルのいずれかが外側で、かつ内側ではない場合、アルファ値の判定が逆向きになる。
    let edge_around_outside_only = edge_overlapped && around_is_any_outside && !around_is_any_inside;

    if is_inside {
        if abs(alpha_counts) > WINDING_THRESHOLD {
            let alpha = clamp(abs(alpha_accum) / (abs(alpha_counts) / UNIT), 0.0, 1.0);
            if edge_around_outside_only {
                return vec4<f32>(color.rgb, 1.0 - alpha);
            } else {
                return vec4<f32>(color.rgb, alpha);
            }
        } else {
            if edge_around_outside_only {
                return vec4<f32>(color.rgb, 0.0);
            } else {
                return vec4<f32>(color.rgb, 1.0);
            }
        }
    } else {
        if abs(alpha_counts) > WINDING_THRESHOLD {
            let alpha = 1.0 - clamp(abs(alpha_accum) / (abs(alpha_counts) / UNIT), 0.0, 1.0);
            return vec4<f32>(color.rgb, alpha);
        } else {
            return vec4<f32>(color.rgb, 0.0);
        }
    }
}
