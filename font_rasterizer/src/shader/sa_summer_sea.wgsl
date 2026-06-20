// 夏の海の波シェーダー
//
// きらめく陽光を浴びた夏の海面を、ゆったりと寄せては返す波で表現します。
// 背景色は uniforms.background_color を基準に、海の色とハイライトを導出するため
// どんな背景色でも自然に馴染みつつコントラストを保ちます。
//
// 利用可能なユニフォーム:
//   uniforms.time             : 起動からの経過秒数 (f32)
//   uniforms.resolution_width : 画面の幅 (f32, pixels)
//   uniforms.resolution_height: 画面の高さ (f32, pixels)
//   uniforms.background_color : 背景色 (vec4<f32>, RGBA)

struct ShaderArtUniforms {
    time: f32,
    resolution_width: f32,
    resolution_height: f32,
    _padding: f32,
    background_color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: ShaderArtUniforms;

// 背景色からのコントラストの強さ (0.0 = 背景色そのまま / 1.0 = 元のフルコントラスト)
// 値を小さくするほど背景色に馴染み、控えめな波になります。
const CONTRAST: f32 = 0.2;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>( 3.0,  1.0),
        vec2<f32>(-1.0,  1.0),
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

// 複数オクターブの value_noise を重ねて、うねる海面の高さ場を作る
// オクターブ数を抑えて 1 ピクセルあたりの sin 計算量を削減している
fn fbm(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var amp = 0.5;
    var freq = p;
    for (var i = 0; i < 3; i++) {
        value += amp * value_noise(freq);
        freq = freq * 2.0;
        amp = amp * 0.5;
    }
    return value;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let resolution = vec2<f32>(uniforms.resolution_width, uniforms.resolution_height);
    let aspect = resolution.x / resolution.y;
    let t = uniforms.time * 0.6;

    var uv = in.uv;
    uv.x *= aspect;

    // 手前ほど波を大きく見せる遠近感
    let depth = mix(2.5, 0.6, in.uv.y);
    let p = vec2<f32>(uv.x, in.uv.y) * vec2<f32>(2.2, 4.0) * depth;

    // 大きなうねり（解析的に勾配を求められる正弦波の重ね合わせ）
    let a = p.y * 3.0 - t * 1.2;
    let b = p.x * 2.0 + p.y * 1.5 + t * 0.8;
    let swell = (sin(a) * 0.5 + 0.5) + sin(b) * 0.25;
    // 細かなさざ波（fbm はピクセルごとに 1 回だけ評価する）
    let ripple = fbm(p * 3.0 + vec2<f32>(t * 0.3, t * 0.6));
    let h = swell * 0.6 + ripple * 0.4;

    // うねりの勾配を解析的に求めて法線を近似する
    // （差分用の追加サンプルが不要になり wave_height の再評価を避けられる）
    let dH = vec2<f32>(
        cos(b) * 0.5,
        cos(a) * 1.5 + cos(b) * 0.375,
    ) * 0.6;
    let normal = normalize(vec3<f32>(-dH.x, -dH.y, 2.0));
    let light_dir = normalize(vec3<f32>(0.3, 0.5, 0.8));
    let diff = clamp(dot(normal, light_dir), 0.0, 1.0);
    let spec = pow(diff, 24.0);

    // 波頭で生まれる白い泡（スパークル）
    let foam = smoothstep(0.82, 0.98, h) * (0.5 + 0.5 * sin(t * 4.0 + p.x * 8.0));

    // 背景色を基準に、海らしい青みのある深色と浅色を導出する
    let bg = uniforms.background_color.rgb;
    let bg_luma = dot(bg, vec3<f32>(0.2126, 0.7152, 0.0722));

    // 海の基調色: 背景に青緑のトーンを混ぜる
    let sea_tint = vec3<f32>(0.05, 0.45, 0.6);
    let deep = mix(bg, sea_tint * 0.5, 0.55);
    let shallow = mix(bg, sea_tint + vec3<f32>(0.1, 0.25, 0.2), 0.65);

    // 高さに応じて深色〜浅色を補間
    var color = mix(deep, shallow, clamp(h, 0.0, 1.0));

    // 拡散光できらめきを乗せる
    color += shallow * diff * 0.25;

    // ハイライト（陽光の反射）の色は背景の明暗に応じてコントラストを確保
    let highlight = select(vec3<f32>(0.05, 0.15, 0.25), vec3<f32>(1.0, 0.98, 0.85), bg_luma < 0.6);
    color = mix(color, highlight, spec * 0.8);

    // 泡を白く加える
    color = mix(color, vec3<f32>(0.95, 0.98, 1.0), foam * 0.6);

    // 背景色との間を CONTRAST で補間し、全体のコントラストを調整する
    color = mix(bg, color, CONTRAST);

    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(color, 1.0);
}
