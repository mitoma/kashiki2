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
    var color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    return vignetting(in.tex_coords, color);
}

fn vignetting(tex_coords: vec2<f32>, color: vec4<f32>) -> vec4<f32> {
    let center = vec2<f32>(0.5, 0.5);
    let distance = length(tex_coords - center);
    let rgb = clamp(color.rgb - vec3<f32>(distance), vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(rgb, color.a);
}
