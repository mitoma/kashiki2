// soralized
let base03 = vec4<f32>(0.0030352699, 0.0241576303, 0.0368894450, 1.0);
let base02 = vec4<f32>(0.0065120910, 0.0368894450, 0.0544802807, 1.0);
let base01 = vec4<f32>(0.0975873619, 0.1559264660, 0.1778884083, 1.0);
let base00 = vec4<f32>(0.1301364899, 0.1980693042, 0.2269658893, 1.0);
let base0  = vec4<f32>(0.2269658893, 0.2961383164, 0.3049873710, 1.0);
let base1  = vec4<f32>(0.2917706966, 0.3564002514, 0.3564002514, 1.0);
let base2  = vec4<f32>(0.8549926877, 0.8069523573, 0.6653873324, 1.0);
let base3  = vec4<f32>(0.9822505713, 0.9215820432, 0.7681512833, 1.0);
let yellow = vec4<f32>(0.4620770514, 0.2501583695, 0.0000000000, 1.0);
let orange = vec4<f32>(0.5972018838, 0.0703601092, 0.0080231922, 1.0);
let red    = vec4<f32>(0.7156936526, 0.0318960287, 0.0284260381, 1.0);
let magent = vec4<f32>(0.6514056921, 0.0368894450, 0.2232279778, 1.0);
let violet = vec4<f32>(0.1499598026, 0.1651322246, 0.5520114899, 1.0);
let blue   = vec4<f32>(0.0193823613, 0.2581829131, 0.6444797516, 1.0);
let cyan   = vec4<f32>(0.0231533647, 0.3564002514, 0.3139887452, 1.0);
let green  = vec4<f32>(0.2345506549, 0.3185468316, 0.0000000000, 1.0);

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

// UNIT = 1.0 / 256.0 
let UNIT :f32 = 0.00390625;
let HARFUNIT: f32 = 0.001953125;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // アルファ成分にテクスチャの重なりの情報を持たせている
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);

    // 奇数かどうかを判定し、奇数なら色をつける
    let odd_color = color.a % (2.0 * UNIT);
    let dist = distance(odd_color, UNIT);
    if UNIT - HARFUNIT < dist && dist < UNIT + HARFUNIT {
        return base03;
    } else {
        return vec4<f32>(color.rgb, 1f);
    }
}