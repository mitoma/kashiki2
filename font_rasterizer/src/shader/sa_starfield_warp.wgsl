// スターフィールドワープシェーダー (Star Wars 風の流れる星)
//
// 星が中心から放射状に飛び出してくる、ハイパースペースワープ風のシェーダーです。
// 色合いは白〜淡い青紫のパステル調です。
//
// 【パフォーマンス設計】
// フラグメントシェーダーのピクセルごとに全 N 星を走査する O(N) の実装は
// 高解像度時に非常に重くなります。
// このシェーダーでは全角度域を NUM_LANES 本のレーンに等分し、
// 各ピクセルは「自分の角度に隣接する 3 レーン」のみチェックする O(1) 手法を採用します。
// 隣接レーン外の星はピクセルに寄与しないため視覚的な差はありません。
//
// 利用可能なユニフォーム:
//   uniforms.time             : 起動からの経過秒数 (f32)
//   uniforms.resolution_width : 画面の幅 (f32, pixels)
//   uniforms.resolution_height: 画面の高さ (f32, pixels)

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

// 疑似乱数 (0.0〜1.0)
fn hash(n: f32) -> f32 {
    return fract(sin(n * 127.1) * 43758.5453);
}

// 1 本のレーンが現在ピクセルに与える輝度と色を返す
fn lane_contribution(uv: vec2<f32>, lane_idx: f32, t: f32) -> vec3<f32> {
    let speed = 0.4 + hash(lane_idx + 100.0) * 0.7;
    let phase = fract(hash(lane_idx + 50.0) * 2.71 + t * speed);

    // レーン中央の角度から放射方向ベクトルを決定
    let lane_angle = (lane_idx + 0.5) / 120.0 * 6.28318 - 3.14159;
    let dir = vec2<f32>(cos(lane_angle), sin(lane_angle));

    let max_radius = 2.2;
    let cur_radius = phase * max_radius;
    let streak_len = phase * phase * 0.18 * speed;
    let tail_radius = max(0.0, cur_radius - streak_len);

    let pos      = dir * cur_radius;
    let tail_pos = dir * tail_radius;

    let seg        = pos - tail_pos;
    let seg_length = length(seg);

    var brightness = 0.0;

    if seg_length > 0.001 {
        let seg_dir    = seg / seg_length;
        let to_uv      = uv - tail_pos;
        let along      = clamp(dot(to_uv, seg_dir), 0.0, seg_length);
        let closest    = tail_pos + seg_dir * along;
        let perp       = length(uv - closest);
        let width      = 0.0015 + phase * 0.003;
        let line_bright = smoothstep(width * 2.5, 0.0, perp);
        let along_fade  = mix(0.15, 1.0, along / seg_length);
        brightness = line_bright * along_fade;
    } else {
        brightness = smoothstep(0.006, 0.0, length(uv - pos));
    }

    brightness *= smoothstep(0.0, 0.06, phase);

    // 淡いパステル調の色 (白〜水色〜薄紫)
    let hue = hash(lane_idx * 3.7);
    let star_color = vec3<f32>(
        0.80 + 0.18 * cos(hue * 6.28),
        0.85 + 0.13 * cos(hue * 6.28 + 2.09),
        0.95 + 0.05 * cos(hue * 6.28 + 4.19)
    );

    return star_color * brightness * 0.45;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let resolution = vec2<f32>(uniforms.resolution_width, uniforms.resolution_height);
    let aspect = resolution.x / resolution.y;
    let t = uniforms.time * 0.35;

    var uv = (in.uv * 2.0 - 1.0);
    uv.x *= aspect;

    var color = vec3<f32>(0.02, 0.02, 0.06);

    // このピクセルの角度から所属レーンを特定し、隣接 3 レーンのみ評価する
    // (120 ループ → 3 ループ、約 40 倍の演算削減)
    let angle        = atan2(uv.y, uv.x);
    let lane_float   = (angle + 3.14159) / 6.28318 * 120.0;
    let center_lane  = floor(lane_float);

    for (var di = -1; di <= 1; di++) {
        let lane_idx = (center_lane + f32(di) + 120.0) % 120.0;
        color += lane_contribution(uv, lane_idx, t);
    }

    color = 1.0 - exp(-color * 1.8);

    return vec4<f32>(color, 1.0);
}
