// Vertex shader
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0)@binding(1)
var s_diffuse: sampler;

// UNIT = 1.0 / 256.0 
const UNIT :f32 = 0.00390625;
const HARFUNIT: f32 = 0.001953125;

// 奇数かどうかを判定する
fn odd_color(tex_coords: vec2<f32>) -> bool {
    let color = textureSample(t_diffuse, s_diffuse, tex_coords);
    let odd_color = color.a % (2.0 * UNIT);
    return UNIT - HARFUNIT < odd_color && odd_color < UNIT + HARFUNIT;
}

// 境界線の判定。袋文字を書きたいときに使いそうなので実装だけ残しておく
// 現在の座標の上下左右のいずれかが描画対象外の場合に true を返す
fn in_border(tex_coords: vec2<f32>) -> bool {
    let texture_size = textureDimensions(t_diffuse);
    let pixel_size = vec2<f32>(1.0 / f32(texture_size.x), 1.0 / f32(texture_size.y));
    let up_odd = odd_color(vec2(tex_coords.x, tex_coords.y - pixel_size.y));
    let down_odd = odd_color(vec2(tex_coords.x, tex_coords.y + pixel_size.y));
    let left_odd = odd_color(vec2(tex_coords.x - pixel_size.x, tex_coords.y));
    let right_odd = odd_color(vec2(tex_coords.x + pixel_size.y, tex_coords.y));
    return !(up_odd && down_odd && left_odd && right_odd);
}

// 境界線のアンチエイリアス
// 現在の座標および上下左右のいずれかが描画対象外の場合に 0.2 ずつ透明度を色をつける。
fn antialias(tex_coords: vec2<f32>) -> f32 {
    let texture_size = textureDimensions(t_diffuse);
    let pixel_size = vec2<f32>(1.0 / f32(texture_size.x), 1.0 / f32(texture_size.y));
    let center_odd = odd_color(tex_coords);
    let up_odd = odd_color(vec2(tex_coords.x, tex_coords.y - pixel_size.y));
    let down_odd = odd_color(vec2(tex_coords.x, tex_coords.y + pixel_size.y));
    let left_odd = odd_color(vec2(tex_coords.x - pixel_size.x, tex_coords.y));
    let right_odd = odd_color(vec2(tex_coords.x + pixel_size.y, tex_coords.y));
    var result = 0.0;
    if center_odd {
        result += 0.2;
    }
    if up_odd {
        result += 0.2;
    }
    if down_odd {
        result += 0.2;
    }
    if left_odd {
        result += 0.2;
    }
    if right_odd {
        result += 0.2;
    }
    return result;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // アルファ成分にテクスチャの重なりの情報を持たせている
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);

    // アンチエイリアスの処理。現在は無効化している。
    //let use_anti_alias = false;
    //if use_anti_alias {
    //    return vec4<f32>(color.rgb, antialias(in.tex_coords));
    //}

    // 奇数かどうかを判定し、奇数なら色をつける
    if odd_color(in.tex_coords) {
        return vec4<f32>(color.rgb, 1f);
    } else {
        return vec4<f32>(0f, 0f, 0f, 0f);
    }
}