// ここはシェーダーで使う便利関数を書くスペース
// soralized
let base03 = vec3<f32>(0.0030352699, 0.0241576303, 0.0368894450);
let base02 = vec3<f32>(0.0065120910, 0.0368894450, 0.0544802807);
let base01 = vec3<f32>(0.0975873619, 0.1559264660, 0.1778884083);
let base00 = vec3<f32>(0.1301364899, 0.1980693042, 0.2269658893);
let base0  = vec3<f32>(0.2269658893, 0.2961383164, 0.3049873710);
let base1  = vec3<f32>(0.2917706966, 0.3564002514, 0.3564002514);
let base2  = vec3<f32>(0.8549926877, 0.8069523573, 0.6653873324);
let base3  = vec3<f32>(0.9822505713, 0.9215820432, 0.7681512833);
let yellow = vec3<f32>(0.4620770514, 0.2501583695, 0.0000000000);
let orange = vec3<f32>(0.5972018838, 0.0703601092, 0.0080231922);
let red    = vec3<f32>(0.7156936526, 0.0318960287, 0.0284260381);
let magent = vec3<f32>(0.6514056921, 0.0368894450, 0.2232279778);
let violet = vec3<f32>(0.1499598026, 0.1651322246, 0.5520114899);
let blue   = vec3<f32>(0.0193823613, 0.2581829131, 0.6444797516);
let cyan   = vec3<f32>(0.0231533647, 0.3564002514, 0.3139887452);
let green  = vec3<f32>(0.2345506549, 0.3185468316, 0.0000000000);

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

    let moved = vec4<f32>(
        model.position.x + (sin(u_buffer.u_time * 5f + f32(model.instance_index) + model.position.x) * model.position.y * 0.5),
        model.position.y, //  * (1f + cos(u_buffer.u_time * 5f + model.position.y / 0.01) * 0.05) * /,
        0.0, //(cos(u_buffer.u_time * 5f + model.position.x / 0.01) * 0.02),
        1.0
    );
    let rotated: vec4<f32> = vec4<f32>(rotate(moved.xyz, u_buffer.u_time * 0.5, vec3<f32>(0.0, 1.0, 0.0)), 1.0);

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
