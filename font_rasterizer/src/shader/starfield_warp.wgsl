// スターフィールドワープシェーダー (Star Wars 風の流れる星)
//
// 星が中心から放射状に飛び出してくる、ハイパースペースワープ風のシェーダーです。
// 色合いは白〜淡い青紫のパステル調です。
//
// 利用可能なユニフォーム:
//   uniforms.time             : 起動からの経過秒数 (f32)
//   uniforms.resolution_width : 画面の幅 (f32, pixels)
//   uniforms.resolution_height: 画面の高さ (f32, pixels)

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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let resolution = vec2<f32>(uniforms.resolution_width, uniforms.resolution_height);
    let aspect = resolution.x / resolution.y;
    let t = uniforms.time * 0.35;

    // アスペクト比補正済みの中心原点 UV (-1..1 程度)
    var uv = (in.uv * 2.0 - 1.0);
    uv.x *= aspect;

    // 暗い宇宙の背景色 (深い藍色)
    var color = vec3<f32>(0.02, 0.02, 0.06);

    // 120 個の星を描画
    for (var i = 0; i < 120; i++) {
        let fi = f32(i);

        // 星ごとにランダムな角度・速度
        let angle = hash(fi) * 6.28318;
        let speed = 0.4 + hash(fi + 100.0) * 0.7;

        // phase: 星が中心から端に向かって 0→1 と進み、端に達するとリセット
        let phase = fract(hash(fi + 50.0) * 2.71 + t * speed);

        // 放射方向ベクトル
        let dir = vec2<f32>(cos(angle), sin(angle));

        // 現在位置と尾の始点
        let max_radius = 2.2;
        let cur_radius = phase * max_radius;
        let pos = dir * cur_radius;

        // フェーズが進むほど尾が長くなる
        let streak_len = phase * phase * 0.18 * speed;
        let tail_pos = dir * max(0.0, cur_radius - streak_len);

        // UV からライン分(pos→tail_pos)への最短距離を計算
        let seg = pos - tail_pos;
        let seg_length = length(seg);

        var brightness = 0.0;

        if seg_length > 0.001 {
            let seg_dir = seg / seg_length;
            let to_uv = uv - tail_pos;
            let along = clamp(dot(to_uv, seg_dir), 0.0, seg_length);
            let closest = tail_pos + seg_dir * along;
            let perp = length(uv - closest);

            // 先端ほど太く明るく、尾に向かって細く暗くなる
            let width = 0.0015 + phase * 0.003;
            let line_bright = smoothstep(width * 2.5, 0.0, perp);
            let along_fade = mix(0.15, 1.0, along / seg_length);
            brightness = line_bright * along_fade;
        } else {
            // 尾が短い（生まれたて）はドットで描画
            brightness = smoothstep(0.006, 0.0, length(uv - pos));
        }

        // 登場直後はフェードイン
        let fade_in = smoothstep(0.0, 0.06, phase);
        brightness *= fade_in;

        // 淡いパステル調の色: 白〜水色〜薄紫のバリエーション
        let hue = hash(fi * 3.7);
        let star_color = vec3<f32>(
            0.80 + 0.18 * cos(hue * 6.28),
            0.85 + 0.13 * cos(hue * 6.28 + 2.09),
            0.95 + 0.05 * cos(hue * 6.28 + 4.19)
        );

        color += star_color * brightness * 0.45;
    }

    // 露出調整 (過飽和を抑えてソフトな見た目に)
    color = 1.0 - exp(-color * 1.8);

    return vec4<f32>(color, 1.0);
}
