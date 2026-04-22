// 淡い雪景色シェーダー
//
// kashikishi の設定ファイル (~/.config/kashikishi/config.json) で
// "background_shader" にこのファイルのパスを指定すると背景に適用されます。

struct ShaderArtUniforms {
    time: f32,
    resolution_width: f32,
    resolution_height: f32,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: ShaderArtUniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 2.0),
        vec2<f32>(2.0, 0.0),
        vec2<f32>(0.0, 0.0),
    );

    var out: VertexOutput;
    out.clip_position = vec4<f32>(positions[vi], 0.0, 1.0);
    out.uv = uvs[vi];
    return out;
}

fn hash21(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let a = hash21(i + vec2<f32>(0.0, 0.0));
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var amp = 0.5;
    var freq = 1.0;

    for (var i = 0; i < 4; i = i + 1) {
        value = value + amp * value_noise(p * freq);
        freq = freq * 2.0;
        amp = amp * 0.5;
    }

    return value;
}

fn snow_layer(uv: vec2<f32>, t: f32, scale: f32, speed: f32, drift: f32) -> f32 {
    var p = uv * scale;
    p.y = p.y - t * speed;
    p.x = p.x + sin((p.y + t * 0.3) * 0.7) * drift;

    let cell = floor(p);
    let local = fract(p) - 0.5;
    let seed = hash21(cell);

    let offset = vec2<f32>(
        hash21(cell + vec2<f32>(12.7, 4.3)) - 0.5,
        hash21(cell + vec2<f32>(7.1, 9.2)) - 0.5,
    ) * 0.7;

    let d = length(local - offset);
    let radius = mix(0.06, 0.18, seed);
    let flake = 1.0 - smoothstep(radius * 0.35, radius, d);
    let twinkle = 0.7 + 0.3 * sin(t * 2.0 + seed * 6.2831853);

    return flake * twinkle;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let resolution = vec2<f32>(uniforms.resolution_width, uniforms.resolution_height);
    let t = uniforms.time;

    let uv = in.uv;
    let centered = vec2<f32>(
        (uv.x * 2.0 - 1.0) * (resolution.x / max(resolution.y, 1.0)),
        uv.y * 2.0 - 1.0,
    );

    let sky_bottom = vec3<f32>(0.62, 0.68, 0.74);
    let sky_top = vec3<f32>(0.74, 0.79, 0.85);
    var color = mix(sky_bottom, sky_top, clamp(uv.y, 0.0, 1.0));

    let mist = fbm(centered * 1.4 + vec2<f32>(0.0, t * 0.03));
    color = color + vec3<f32>(0.02, 0.02, 0.03) * (mist - 0.5);

    let ground_mask = 1.0 - smoothstep(0.28, 0.46, uv.y);
    let ground_tex = fbm(vec2<f32>(uv.x * 5.0, uv.y * 18.0) + vec2<f32>(0.0, t * 0.02));
    let ground_color = vec3<f32>(0.78, 0.83, 0.88) + vec3<f32>(ground_tex * 0.03);
    color = mix(color, ground_color, ground_mask);

    let snow_uv = vec2<f32>(centered.x, uv.y);
    let snow_far = snow_layer(snow_uv, t, 14.0, 0.06, 0.15) * 0.35;
    let snow_mid = snow_layer(snow_uv, t, 24.0, 0.12, 0.25) * 0.45;
    let snow_near = snow_layer(snow_uv, t, 38.0, 0.22, 0.40) * 0.65;
    let snow = snow_far + snow_mid + snow_near;

    color = color + vec3<f32>(snow * 0.55);

    //let vignette = smoothstep(1.45, 0.35, length(centered));
    //color = mix(vec3<f32>(0.57, 0.62, 0.68), color, vignette);

    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
