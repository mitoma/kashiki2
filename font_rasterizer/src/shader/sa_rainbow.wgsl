// シェーダーアートテンプレート
//
// このファイルをコピーして独自のシェーダーアートを作成してください。
// kashikishi の設定ファイル (~/.config/kashikishi/config.json) で
// "background_shader" にこのファイルのパスを指定することで背景に適用されます。
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

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// 頂点シェーダー: フルスクリーン三角形を生成する
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    // フルスクリーンをカバーする三角形の頂点
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

// フラグメントシェーダー: ここを自由に書き換えてください
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let resolution = vec2<f32>(uniforms.resolution_width, uniforms.resolution_height);
    let uv = in.uv;
    let t = uniforms.time;

    // サンプル: 時間とともに変化するグラデーション
    let r = 0.5 + 0.5 * sin(t + uv.x * 3.14159);
    let g = 0.5 + 0.5 * sin(t * 0.7 + uv.y * 3.14159);
    let b = 0.5 + 0.5 * cos(t * 1.3);

    return vec4<f32>(r, g, b, 1.0);
}
