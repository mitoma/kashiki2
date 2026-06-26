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
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    // 重なり回数テクスチャから値を取得（Rgba16Float: 符号付き浮動小数点）
    let overlap_count = textureSample(t_overlap_count, s_diffuse, in.tex_coords);

    let winding = overlap_count.r;
    let alpha_accum = overlap_count.g;
    let alpha_counts = overlap_count.b;

    // Non-Zero Winding Rule: winding が非ゼロなら内側
    let is_inside = abs(winding) > WINDING_THRESHOLD;

    if is_inside {
        if abs(alpha_counts) > WINDING_THRESHOLD {
            // エッジ付近: アルファで滑らかに
            let alpha = clamp(abs(alpha_accum) / (abs(alpha_counts) / UNIT), 0.0, 1.0);
            return vec4<f32>(color.rgb, alpha);
        } else {
            return vec4<f32>(color.rgb, 1.0);
        }
    } else {
        if abs(alpha_counts) > WINDING_THRESHOLD {
            let alpha = 1.0 - clamp(abs(alpha_accum) / (abs(alpha_counts) / UNIT), 0.0, 1.0);
            if alpha > 0.001 {
                return vec4<f32>(color.rgb, alpha);
            }
        }
        return vec4<f32>(color.rgb, 0.0);
    }
}
