struct FragmentOutput {
    [[location(0)]] f_color: vec4<f32>;
};

[[group(0), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(0), binding(1)]]
var s_diffuse: sampler;

[[stage(fragment)]]
fn main([[location(0)]] v_tex_corrds: vec2<f32>) -> FragmentOutput {
    let font_color: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0); // とりあえず今は黒
    let font_alpha: f32 = textureSample(t_diffuse, s_diffuse, v_tex_corrds).x;
    let f_color: vec4<f32> = vec4<f32>(font_color, font_alpha);
    return FragmentOutput(f_color);
}
