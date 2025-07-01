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
@group(0) @binding(1)
var s_diffuse: sampler;
// 型は f32 だが直近の奇数/偶数判定を u32 で持つ
@group(0) @binding(2)
var t_history_bits: texture_storage_2d<r32uint, read_write>;

struct Uniforms {
    frame_count: u32,
};

@group(0) @binding(3)
var<uniform> u_buffer: Uniforms;

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

/// f32を u32 のビット列として取得
fn f32_to_bits(value: f32) -> u32 {
    return bitcast<u32>(value);
}

/// u32 のビット列を f32 として解釈
fn bits_to_f32(bits: u32) -> f32 {
    return bitcast<f32>(bits);
}

// u32 のビット列から立っているビットの数をカウントし n/32 の f32 値を返す
fn count_bits(bits: u32) -> f32 {
    return f32(countOneBits(bits)) / 32.0;
}

// u32 のビット列の指定ビットの値を変更
fn set_bit(bits: u32, bit_index: u32, value: bool) -> u32 {
    if value {
        return bits | (1u << bit_index);
    } else {
        return bits & ~(1u << bit_index);
    }
}

/// vec4<f32> の指定した要素をビット列として取得
fn get_component_bits(v: vec4<f32>, component: u32) -> u32 {
    if component == 0u {
        return bitcast<u32>(v.x);
    } else if component == 1u {
        return bitcast<u32>(v.y);
    } else if component == 2u {
        return bitcast<u32>(v.z);
    } else {
        return bitcast<u32>(v.w);
    }
}

/// vec4<f32> の指定した要素にビット列から設定
fn set_component_from_bits(v: vec4<f32>, component: u32, bits: u32) -> vec4<f32> {
    var result = v;
    let float_value = bitcast<f32>(bits);
    if component == 0u {
        result.x = float_value;
    } else if component == 1u {
        result.y = float_value;
    } else if component == 2u {
        result.z = float_value;
    } else {
        result.w = float_value;
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

    // テンポラルアンチエイリアスを有効にする場合
    let use_temporal_aa = true;
    if use_temporal_aa {
        let current_odd = odd_color(in.tex_coords);

        let ipos: vec2<i32> = vec2<i32>(floor(in.clip_position.xy));
        var pixel_value = textureLoad(t_history_bits, ipos);
        var odd_history_bits = pixel_value.r;
        odd_history_bits = set_bit(odd_history_bits, u_buffer.frame_count % 32, current_odd);
        pixel_value.r = odd_history_bits;
        textureStore(t_history_bits, ipos, pixel_value);
        return vec4<f32>(color.rgb, f32(countOneBits(odd_history_bits)) / 32.0);
    }

    // 奇数かどうかを判定し、奇数なら色をつける
    if odd_color(in.tex_coords) {
        return vec4<f32>(color.rgb, 1f);
    } else {
        return vec4<f32>(0f, 0f, 0f, 0f);
    }
}