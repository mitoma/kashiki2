// Vertex shader

struct VertexInput {
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
    var out: VertexOutput;
    out.color = model.color;
    out.clip_position = vec4<f32>(model.position, 1.0);
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