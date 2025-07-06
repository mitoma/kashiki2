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
    let pos = ipos.x + ipos.y * u_buffer.u_width * 4u;
    let counts0 = overlap_count_bits[pos];
    let counts1 = overlap_count_bits[pos + 1];
    let counts2 = overlap_count_bits[pos + 2];
    let counts3 = overlap_count_bits[pos + 3];

    var alpha: f32 = 0.0;

    if counts0 % 2u == 1u {
        alpha += 0.25;
    }
    if counts1 % 2u == 1u {
        alpha += 0.25;
    }
    if counts2 % 2u == 1u {
        alpha += 0.25;
    }
    if counts3 % 2u == 1u {
        alpha += 0.25;
    }

    if alpha > 0.0 {
        return vec4<f32>(color.rgb, alpha);
    } else {
        return vec4<f32>(0f, 0f, 0f, 0f);
    }
}
