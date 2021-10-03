[[block]]
struct Uniforms {
    u_view_proj: mat4x4<f32>;
    u_time: f32;
};

[[block]]
struct Instances {
    s_models: [[stride(64)]] array<mat4x4<f32>>;
};

struct VertexOutput {
    [[location(0)]] v_tex_coords: vec2<f32>;
    [[builtin(position)]] member: vec4<f32>;
};

[[group(1), binding(0)]]
var<uniform> u_buffer: Uniforms;
[[group(1), binding(1)]]
var<storage, read> i_buffer: Instances;

fn rotate(p: vec3<f32>, angle: f32, axis: vec3<f32>) -> vec3<f32> {
    let a: vec3<f32> = normalize(axis);
    let s: f32 = sin(angle);
    let c: f32 = cos(angle);
    let r: f32 = 1.0 - c;
    let m: mat3x3<f32> = mat3x3<f32>(
        vec3<f32>(
          a.x * a.x * r + c,
          a.y * a.x * r + a.z * s,
          a.z * a.x * r - a.y * s),
        vec3<f32>(
          a.x * a.y * r - a.z * s,
          a.y * a.y * r + c,
          a.z * a.y * r + a.x * s),
        vec3<f32>(
          a.x * a.z * r + a.y * s,
          a.y * a.z * r - a.x * s,
          a.z * a.z * r + c)
    );
    return m * p;
}

[[stage(vertex)]]
fn main(
    [[location(0)]] a_position: vec3<f32>,
    [[location(1)]] a_tex_coords: vec2<f32>,
    [[builtin(instance_index)]] instance_index: u32,
) -> VertexOutput {
    let rotated: vec4<f32> = vec4<f32>(rotate(a_position, u_buffer.u_time, vec3<f32>(0.0, 1.0, 0.0)), 1.0);
    let position: vec4<f32> = u_buffer.u_view_proj * i_buffer.s_models[instance_index] * rotated;
    return VertexOutput(a_tex_coords, position);
}
