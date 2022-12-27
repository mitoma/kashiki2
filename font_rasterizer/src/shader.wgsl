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

let UNIT :f32 = 0.00390625;
let HARFUNIT: f32 = 0.001953125;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 赤色成分にテクスチャの重なりの情報を持たせている
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords).r;

    // 奇数のかどうかを判定し、奇数なら色をつける
    let odd_color = color % (2.0 * UNIT);
    let dist = distance(odd_color, UNIT);
    if UNIT - HARFUNIT < dist && dist < UNIT + HARFUNIT {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    } else {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }
}