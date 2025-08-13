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
var<storage, read_write> overlap_count_bits: array<u32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // テクスチャから色を取得
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);

    // 前のステージで取得したオーバーラップ数を取得
    let ipos: vec2<u32> = vec2<u32>(floor(in.clip_position.xy));
    let pos = (ipos.x + ipos.y * u_buffer.u_width) * 3u;
    let alpha_total = pos + 1u;
    let alpha_count = pos + 2u;

    let counts = overlap_count_bits[pos];

    // 奇数かどうかを判定し、奇数なら色をつける
    if counts % 2u == 1u {
        return vec4<f32>(color.rgba);
    } else {
        return vec4<f32>(0f, 0f, 0f, 0f);
    }
}
