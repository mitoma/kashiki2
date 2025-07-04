struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main() -> VertexOutput {
    var out: VertexOutput;
    return out;
}

// Fragment shader
@fragment
fn fs_main() -> @location(0) vec4<u32> {
    return vec4<u32>(0, 0, 0, 0);
}