// ここはシェーダーで使う便利関数を書くスペース
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

// Vertex shader
struct Uniforms {
    u_view_proj: mat4x4<f32>,
    u_time: f32,
};

@group(0) @binding(0)
var<uniform> u_buffer: Uniforms;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec2<f32>,
    @location(1) wait: vec2<f32>,
};

struct InstancesInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    instances: InstancesInput,
) -> VertexOutput {
    let instance_matrix = mat4x4<f32>(
        instances.model_matrix_0,
        instances.model_matrix_1,
        instances.model_matrix_2,
        instances.model_matrix_3,
    );

    let moved = vec4<f32>(
        model.position.x + (sin(u_buffer.u_time * 5f + f32(model.instance_index) + model.position.x) * model.position.y * 0.5),
        model.position.y, //  * (1f + cos(u_buffer.u_time * 5f + model.position.y / 0.01) * 0.05) * /,
        0.0, //(cos(u_buffer.u_time * 5f + model.position.x / 0.01) * 0.02),
        1.0
    );
    let rotated: vec4<f32> = vec4<f32>(rotate(moved.xyz, u_buffer.u_time * 0.5, vec3<f32>(0.0, 1.0, 0.0)), 1.0);

    var out: VertexOutput;
    out.color = vec3<f32>(1f, model.wait.xy);
    out.clip_position = u_buffer.u_view_proj * instance_matrix * moved;
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
