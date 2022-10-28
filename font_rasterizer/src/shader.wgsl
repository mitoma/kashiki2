// Vertex shader

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) wait: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) wait: vec2<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.wait = model.wait;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
//    let in_bezier = pow((in.wait.x / 2.0 + in.wait.y), 2.0) < in.wait.y;
//    if !in_bezier {
//        discard;
//    }
    return vec4<f32>(0.0, 1.0, 0.0, 1.0);
}