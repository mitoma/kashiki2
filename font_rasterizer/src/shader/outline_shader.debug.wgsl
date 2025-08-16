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

const UNIT :f32 = 0.00390625;
const HARFUNIT: f32 = 0.001953125;
const ALPHA_STEP: f32 = 64f;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // テクスチャから色を取得
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    // 重なり回数テクスチャから値を取得
    let overlap_count = textureSample(t_overlap_count, s_diffuse, in.tex_coords);

    let count_value = overlap_count.r;
    let alpha_count = overlap_count.g;
    let counts = u32(count_value * 255.0 + HARFUNIT); // 8ビットから整数に復元

    let alpha = clamp(alpha_count / f32(counts) * ALPHA_STEP, 0.0, 1.0);

    // 奇数かどうかを判定し、奇数なら色をつける
    if counts % 2u == 1u {
        return vec4<f32>(color.rgb, alpha);
    } else {
        if alpha > UNIT {
            return vec4<f32>(color.rgb, 1f - alpha);
        } else {
            return vec4<f32>(0f, 0f, 0f, 0f);
        }
    }
}
