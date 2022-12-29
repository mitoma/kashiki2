// Vertex shader
struct Uniforms {
    u_view_proj: mat4x4<f32>,
    u_time: f32,
};

struct Instances {
    s_models: array<mat4x4<f32>>,
};

@group(0) @binding(0)
var<uniform> u_buffer: Uniforms;
@group(0) @binding(1)
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
            a.z * a.x * r - a.y * s
        ),
        vec3<f32>(
            a.x * a.y * r - a.z * s,
            a.y * a.y * r + c,
            a.z * a.y * r + a.x * s
        ),
        vec3<f32>(
            a.x * a.z * r + a.y * s,
            a.y * a.z * r - a.x * s,
            a.z * a.z * r + c
        )
    );
    return m * p;
}

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    let moved = vec4<f32>(
        model.position.x * (1f + sin(u_buffer.u_time * 5f + model.position.x / 0.01) * 0.05),
        model.position.y * (1f + cos(u_buffer.u_time * 5f + model.position.y / 0.01) * 0.05),
        model.position.z + (cos(u_buffer.u_time * 5f + model.position.x / 0.01) * 0.02),
        1.0
    );
    let rotated: vec4<f32> = vec4<f32>(rotate(moved.xyz, u_buffer.u_time * 0.5, vec3<f32>(0.0, 1.0, 0.0)), 1.0);

    var out: VertexOutput;
    out.color = model.color;
    out.clip_position = u_buffer.u_view_proj * i_buffer.s_models[model.instance_index] * rotated;
    return out;
}

let UNIT :f32 = 0.00390625;
// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // R = ポリゴンの重なった数
    // G, B = ベジエ曲線

    // B が 0 の時はベジエ曲線ではなくて単なるポリゴンとして処理する
    //if in.color.b == 0.0 {
        //return vec4<f32>(in.color, UNIT);
    //}

    // G, B のいずれかが 0 でないとき
    let in_bezier = pow((in.color.g / 2.0 + in.color.b), 2.0) < in.color.b;
    if !in_bezier {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    return vec4<f32>(in.color, UNIT);
}