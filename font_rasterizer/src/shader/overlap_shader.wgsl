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
    @location(9) color: vec3<f32>,
    @location(10) motion: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) wait: vec3<f32>,
    @location(1) color: vec3<f32>,
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

    let gain = (sin(u_buffer.u_time * 5f + model.position.x) * model.position.y * 0.2);
    var x_gain: f32 = 0f;
    var y_gain: f32 = 0f;
    var z_gain: f32 = 0f;
    var x_rotate: f32 = 0f;
    var y_rotate: f32 = 0f;
    var z_rotate: f32 = 0f;

    if (0x1u & instances.motion) == 0x1u {
        x_gain += gain;
    }
    if (0x2u & instances.motion) == 0x2u {
        y_gain += gain;
    }
    if (0x4u & instances.motion) == 0x4u {
        z_gain += gain;
    }
    if (0x8u & instances.motion) == 0x8u {
        x_rotate = 1.0f;
    }
    if (0x10u & instances.motion) == 0x10u {
        y_rotate = 1.0f;
    }
    if (0x20u & instances.motion) == 0x20u {
        z_rotate = 1.0f;
    }

    var moved = vec4<f32>(
        model.position.x + x_gain,
        model.position.y + y_gain,
        0.0 + z_gain,
        1.0
    );

    if x_rotate != 0f || y_rotate != 0f || z_rotate != 0f {
        moved = vec4<f32>(rotate(moved.xyz, sin(u_buffer.u_time) * distance(moved.xy, vec2<f32>(0f, 0f)) * 3f, vec3<f32>(x_rotate, y_rotate, z_rotate)), 1.0);
    }

    var out: VertexOutput;
    out.wait = vec3<f32>(1f, model.wait.xy);
    out.color = instances.color;
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
    let in_bezier = pow((in.wait.g / 2.0 + in.wait.b), 2.0) < in.wait.b;
    if !in_bezier {
        return vec4<f32>(in.color, 0.0);
    }
    return vec4<f32>(in.color, UNIT);
}
