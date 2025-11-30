// ここはシェーダーで使う便利関数を書くスペース
const PI: f32 = 3.14159265359;
const HALF_PI: f32 = 1.57079632679;
const DOUBLE_PI: f32 = 6.28318530718;

const EASING_LINER: u32 = 0u;
const EASING_SIN: u32 = 1u;
const EASING_QUAD: u32 = 2u;
const EASING_CUBIC: u32 = 3u;
const EASING_QUART: u32 = 4u;
const EASING_QUINT: u32 = 5u;
const EASING_EXPO: u32 = 6u;
const EASING_CIRC: u32 = 7u;
const EASING_BACK: u32 = 8u;
const EASING_ELASTIC: u32 = 9u;
const EASING_BOUNCE: u32 = 10u;

const BOUNCE_N1: f32 = 7.5625;
const BOUNCE_D1: f32 = 2.75;
const BACK_C1: f32 = 1.70158;
const BACK_C3: f32 = 2.70158;
const EXPO: f32 = 2.09439510239;

/// value の n ビット目が立っているかどうか調べる
/// n は 0 - 31 の範囲
fn bit_check(value: u32, n: u32) -> bool {
    return (value & (1u << n)) != 0u;
}

/// 0000_0000_0000_0000_0000_0000_0000_0000
/// u32 から一部の値を数値として取ってくる
/// upper 
fn bit_range(value: u32, upper: u32, lower: u32) -> u32 {
    let left_shift = 31u - upper;
    let right_shift = left_shift + lower;
    return (value << left_shift) >> right_shift;
}

// easing function
fn internal_easing_func(value: f32, easing_type: u32) -> f32 {
    if easing_type == EASING_LINER {
        return value;
    } else if easing_type == EASING_SIN {
        return 1f - cos(value * HALF_PI);
    } else if easing_type == EASING_QUAD {
        return value * value;
    } else if easing_type == EASING_CUBIC {
        return value * value * value;
    } else if easing_type == EASING_QUART {
        return value * value * value * value;
    } else if easing_type == EASING_QUINT {
        return value * value * value * value * value;
    } else if easing_type == EASING_EXPO {
        return pow(2f, 10f * value - 10f);
    } else if easing_type == EASING_CIRC {
        return 1f - sqrt(1f - pow(value, 2f));
    } else if easing_type == EASING_BACK {
        return BACK_C3 * value * value * value - BACK_C1 * value * value;
    } else if easing_type == EASING_ELASTIC {
        return -pow(2f, 10f * value - 10f) * sin((value * 10f - 10.75) * EXPO);
    } else if easing_type == EASING_BOUNCE {
        let x = 1f - value;
        if x < 1f / BOUNCE_D1 {
            return 1f - (BOUNCE_N1 * x * x);
        } else if x < 2f / BOUNCE_D1 {
            let x = x - (1.5 / BOUNCE_D1);
            return 1f - (BOUNCE_N1 * x * x + 0.75);
        } else if x < 2.5f / BOUNCE_D1 {
            let x = x - (2.25 / BOUNCE_D1);
            return 1f - (BOUNCE_N1 * x * x + 0.9375);
        } else {
            let x = x - (2.625 / BOUNCE_D1);
            return 1f - (BOUNCE_N1 * x * x + 0.984375);
        }
    }
    // fallback liner
    return value;
}

fn easing_function(value: f32, easing_type: u32, ease_in: bool, ease_out: bool) -> f32 {
    if value <= 0f {
        return 0f;
    } else if value >= 1f {
        return 1f;
    } else if ease_in && ease_out {
        if value < 0.5 {
            return internal_easing_func(value * 2f, easing_type) / 2f;
        } else {
            let value = 1f - ((value - 0.5f) * 2f);
            let result = internal_easing_func(value, easing_type);
            return (1f - result) / 2f + 0.5f;
        }
    } else if ease_in {
        return internal_easing_func(value, easing_type);
    } else if ease_out {
        return 1f - internal_easing_func(1f - value, easing_type);
    } else {
        return value;
    }
}

fn rotate(p: vec3<f32>, angle: f32, axis: vec3<f32>) -> vec3<f32> {
    let a: vec3<f32> = normalize(axis);
    let s: f32 = sin(angle);
    let c: f32 = cos(angle);
    let r: f32 = 1.0 - c;
    let m: mat3x3<f32> = mat3x3<f32>(
        vec3<f32>(
            a.x * a.x * r + c,
            a.y * a.x * r + a.z * s,
            a.z * a.x * r - a.y * s
        ),
        vec3<f32>(
            a.x * a.y * r - a.z * s,
            a.y * a.y * r + c,
            a.z * a.y * r + a.x * s
        ),
        vec3<f32>(
            a.x * a.z * r + a.y * s,
            a.y * a.z * r - a.x * s,
            a.z * a.z * r + c
        )
    );
    return m * p;
}

// Vertex shader
struct Uniforms {
    u_view_proj: mat4x4<f32>,
    u_default_view_proj: mat4x4<f32>,
    u_time: u32,
    u_width: u32,
    u_antialiasing: u32,
};

@group(0) @binding(0)
var<uniform> u_buffer: Uniforms;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec2<f32>,
    // 座標の種別
    //
    // 原点B  : 0
    // 始点B  : 2
    // 終点B  : 4
    // 原点L  : 1
    // 始点L  : 3
    // 終点L  : 5
    // 制御点 : 6
    @location(1) vertex_type: u32,
};

struct InstancesInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) color: vec3<f32>,
    @location(10) motion: u32,
    @location(11) start_time: u32,
    @location(12) gain: f32,
    @location(13) duration: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    // フラグメントシェーダーで3つの各頂点からほかの頂点に向かって減衰するウエイト値を保持する
    // x は原点または制御点からのウエイト、y と z は始点または終点からのウエイトとなる
    // 
    // 原点B  (1.0, 0.0, 0.0)
    // 始点B  (0.0, 1.0, 0.0)
    // 終点B  (0.0, 0.0, 1.0)
    // 原点L  (1.0, 0.0, 0.0)
    // 始点L  (0.0, 1.0, 0.0)
    // 終点L  (0.0, 0.0, 1.0)
    // 制御点 (1.0, 0.0, 0.0)
    @location(0) color: vec3<f32>,
    @location(1) wait: vec3<f32>,
    // フラグメントシェーダーで三角形の種別を判定するために使う。
    // 
    // 座標はベジエ曲線部、ベジエ補助直線、直線で共用されるため、フラグメントシェーダー内での判定に個のアトリビュートが必要となる。
    // 
    // フラグメントシェーダ内で X が 1 の場合はベジエ曲線部、Y が 1 の場合はベジエ補助直線、Z が 1 の場合は直線として扱う。
    // ベジエ曲線部:   (1.0, 0.0, 0.0)
    // ベジエ補助直線: (0.0, 1.0, 0.0)
    // 直線:           (0.0, 0.0, 1.0)
    // 
    // triangle_type に従って以下の値になる。
    // 原点B  (0.0, 1.0, 0.0)
    // 始点B  (1.0, 1.0, 0.0)
    // 終点B  (1.0, 1.0, 0.0)
    // 原点L  (0.0, 0.0, 1.0)
    // 始点L  (0.0, 0.0, 1.0)
    // 終点L  (0.0, 0.0, 1.0)
    // 制御点 (1.0, 0.0, 0.0)
    @location(2) triangle_type: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    instances: InstancesInput,
) -> VertexOutput {
    let instance_matrix = mat4x4<f32>(
        instances.model_matrix_0,
        instances.model_matrix_1,
        instances.model_matrix_2,
        instances.model_matrix_3,
    );
    let motion = instances.motion;
    let has_motion = bit_check(instances.motion, 31u);
    let ease_in = bit_check(instances.motion, 30u);
    let ease_out = bit_check(instances.motion, 29u);
    let is_loop = bit_check(motion, 28u);

    // motion detail
    var to_current = bit_check(motion, 27u);
    let turn_back = bit_check(motion, 26u);
    let use_x_distance = bit_check(motion, 25u);
    let use_y_distance = bit_check(motion, 24u);
    let use_xy_distance = bit_check(motion, 23u);

    let ignore_camera = bit_check(motion, 20u);

    let easing_type = bit_range(motion, 19u, 16u);
    let duration = instances.duration;
    let gain = instances.gain;
    let x_distance = model.position.x;
    let y_distance = model.position.y;
    let xy_distance = distance(model.position.xy, vec2<f32>(0f, 0f));

    var v = 0f;
    var easing_position = u_buffer.u_time - instances.start_time;
    let harf_duration = duration / 2u;
    if turn_back {
        if easing_position > duration {
            easing_position = 0u;
        } else if easing_position <= harf_duration {
            easing_position = easing_position * 2u;
        } else {
            easing_position = duration - ((easing_position - harf_duration) * 2u);
        }
    } else if is_loop {
        if (easing_position / duration) % 2u == 1u {
            to_current = !to_current;
        }
        easing_position = easing_position % duration;
    }

    if easing_position <= 0u {
        v = 0f;
    } else if easing_position >= duration {
        v = 1f;
    } else {
        v = f32(easing_position) / f32(duration);
    }

    var calced_gain = 0f;
    if to_current {
        // to_current が true の場合は動作が収束の方向に向かう
        calced_gain = easing_function(1f - v, easing_type, ease_out, ease_in) * gain;
    } else {
        calced_gain = easing_function(v, easing_type, ease_in, ease_out) * gain;
    }

    if use_x_distance {
        calced_gain = calced_gain * x_distance;
    }
    if use_y_distance {
        calced_gain = calced_gain * y_distance;
    }
    if use_xy_distance {
        calced_gain = calced_gain * xy_distance;
    }

    var x_gain: f32 = 0f;
    var y_gain: f32 = 0f;
    var z_gain: f32 = 0f;
    var x_rotate: f32 = 0f;
    var y_rotate: f32 = 0f;
    var z_rotate: f32 = 0f;
    var strech_x: f32 = 1f;
    var strech_y: f32 = 1f;

    if bit_check(motion, 0u) {
        x_gain += calced_gain;
    }
    if bit_check(motion, 1u) {
        x_gain -= calced_gain;
    }
    if bit_check(motion, 2u) {
        y_gain += calced_gain;
    }
    if bit_check(motion, 3u) {
        y_gain -= calced_gain;
    }
    if bit_check(motion, 4u) {
        z_gain += calced_gain;
    }
    if bit_check(motion, 5u) {
        z_gain -= calced_gain;
    }
    if bit_check(motion, 6u) {
        x_rotate = 1.0f;
    }
    if bit_check(motion, 7u) {
        x_rotate = -1.0f;
    }
    if bit_check(motion, 8u) {
        y_rotate = 1.0f;
    }
    if bit_check(motion, 9u) {
        y_rotate = -1.0f;
    }
    if bit_check(motion, 10u) {
        z_rotate = 1.0f;
    }
    if bit_check(motion, 11u) {
        z_rotate = -1.0f;
    }
    if bit_check(motion, 12u) {
        strech_x += calced_gain;
    }
    if bit_check(motion, 13u) {
        strech_x -= calced_gain;
    }
    if bit_check(motion, 14u) {
        strech_y += calced_gain;
    }
    if bit_check(motion, 15u) {
        strech_y -= calced_gain;
    }

    var moved = vec4<f32>(
        (model.position.x * strech_x) + x_gain,
        (model.position.y * strech_y) + y_gain,
        0.0 + z_gain,
        1.0
    );

    if x_rotate != 0f || y_rotate != 0f || z_rotate != 0f {
        moved = vec4<f32>(rotate(moved.xyz, calced_gain * DOUBLE_PI, vec3<f32>(x_rotate, y_rotate, z_rotate)), 1.0);
    }

    var out: VertexOutput;

    if model.vertex_type == 0u {
        // 原点B
        out.wait = vec3<f32>(1.0, 0.0, 0.0);
        out.triangle_type = vec3<f32>(0.0, 1.0, 0.0);
    } else if model.vertex_type == 1u {
        // 原点L
        out.wait = vec3<f32>(1.0, 0.0, 0.0);
        out.triangle_type = vec3<f32>(0.0, 0.0, 1.0);
    } else if model.vertex_type == 2u {
        // 始点B
        out.wait = vec3<f32>(0.0, 1.0, 0.0);
        out.triangle_type = vec3<f32>(1.0, 1.0, 0.0);
    } else if model.vertex_type == 3u {
        // 始点L
        out.wait = vec3<f32>(0.0, 1.0, 0.0);
        out.triangle_type = vec3<f32>(0.0, 0.0, 1.0);
    } else if model.vertex_type == 4u {
        // 終点B
        out.wait = vec3<f32>(0.0, 0.0, 1.0);
        out.triangle_type = vec3<f32>(1.0, 1.0, 0.0);
    } else if model.vertex_type == 5u {
        // 終点L
        out.wait = vec3<f32>(0.0, 0.0, 1.0);
        out.triangle_type = vec3<f32>(0.0, 0.0, 1.0);
    } else if model.vertex_type == 6u {
        // 制御点
        out.wait = vec3<f32>(1.0, 0.0, 0.0);
        out.triangle_type = vec3<f32>(1.0, 0.0, 0.0);
    }

    out.color = instances.color;
    if ignore_camera {
        out.clip_position = u_buffer.u_default_view_proj * instance_matrix * moved;
    } else {
        out.clip_position = u_buffer.u_view_proj * instance_matrix * moved;
    }
    return out;
}

// 最小限の Vertex Shader
// ただし vs_main と比べてもさほどパフォーマンス改善には寄与しなさそう
@vertex
fn vs_main_minimum(
    model: VertexInput,
    instances: InstancesInput,
) -> VertexOutput {
    let instance_matrix = mat4x4<f32>(
        instances.model_matrix_0,
        instances.model_matrix_1,
        instances.model_matrix_2,
        instances.model_matrix_3,
    );

    let motion = instances.motion;
    let ignore_camera = bit_check(motion, 20u);

    var moved = vec4<f32>(model.position.x, model.position.y, 0.0, 1.0);

    var out: VertexOutput;
    out.color = instances.color;
    if ignore_camera {
        out.clip_position = u_buffer.u_default_view_proj * instance_matrix * moved;
    } else {
        out.clip_position = u_buffer.u_view_proj * instance_matrix * moved;
    }
    return out;
}

// マルチターゲット用のフラグメントシェーダーアウトプット
struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) count: vec4<f32>,
}

const UNIT :f32 = 0.00390625;
const ALPHA_STEP: f32 = 16f;

const NEAR_ZERO = 1e-6;
const NEAR_ONE = 1.0 - 1e-6;

fn near_eq_zero(value: f32) -> bool {
    return value < NEAR_ZERO;
}

fn near_eq_one(value: f32) -> bool {
    return value > NEAR_ONE;
}

fn in_naive_range(value: f32) -> bool {
    return value >= 0.0 && value <= 1.0;
}

fn under_one(value: f32) -> bool {
    return value <= 1.0;
}

fn greater_than_zero(value: f32) -> bool {
    return value >= 0.0;
}

// smoothstep のリニア版
fn linerstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    return clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
}

// Fragment shader (マルチターゲット版)
@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let enable_antialiasing = u_buffer.u_antialiasing != 0u;
    let is_bezier_pre = near_eq_one(in.triangle_type.x);
    let is_bezier_line_pre = near_eq_one(in.triangle_type.y);
    let is_line_pre = near_eq_one(in.triangle_type.z);

    let is_bezier = is_bezier_pre && !is_bezier_line_pre && !is_line_pre;
    let is_bezier_line = is_bezier_line_pre && !is_bezier_pre && !is_line_pre;
    let is_line = is_line_pre && !is_bezier_pre && !is_bezier_line_pre;

    var output: FragmentOutput;
    output.color = vec4<f32>(in.color.rgb, 1f);

    // 処理の内容的には以降の if 文の中で行えば済む処理だが
    // WebGPU は fwidth は実行パスの分岐先でだけ呼び出されると正しい結果を返せないとエラーを返す実装があるのでここで実行する。

    // Bezier curve のSDF距離計算
    let bezier_distance = pow((in.wait.x * 0.5 + in.wait.y), 2.0) - in.wait.y;
    // 隣接ピクセルの距離との差分
    let bezier_distance_fwidth = fwidth(bezier_distance);

    // 直線のSDF距離計算
    let triangle_distance = in.wait.x;
    // 隣接ピクセルの距離との差分
    let triangle_distance_fwidth = fwidth(triangle_distance);

    if is_bezier {
        // Bezier curveの場合の処理
        // linerstep は 0.0->1.0 に変化するので、1.0-linerstep で 1.0->0.0 に反転
        var alpha = 1.0 - linerstep(-bezier_distance_fwidth / 2.0, bezier_distance_fwidth / 2.0, bezier_distance);
        if !enable_antialiasing && !near_eq_zero(alpha) {
            alpha = 1.0;
        }

        if alpha > 0.0 {
            output.count.r = UNIT;
            if !near_eq_one(alpha) {
                output.count.g = alpha / ALPHA_STEP;
                output.count.b = UNIT;
            }
        } else {
            discard;
        }
    //} else if true {
    } else {
        // 三角形の場合の処理
        var alpha = linerstep(-triangle_distance_fwidth / 2.0, triangle_distance_fwidth / 2.0, triangle_distance);
        if !enable_antialiasing {
            alpha = 1.0;
        }

        // ベジエの補完的直線
        if is_bezier_line {
            if (in_naive_range(in.wait.x)) && (in_naive_range(in.wait.y)) && (in_naive_range(in.wait.z)) {
                output.count.r = UNIT;
            }
        } else if is_line {
            if (in_naive_range(in.wait.y)) && (in_naive_range(in.wait.z)) {
                output.count.r = UNIT;
                if !near_eq_one(alpha) {
                    output.count.g = alpha / ALPHA_STEP;
                    output.count.b = UNIT;
                }
            }
        } else {
            discard;
        }
    }
    return output;
}
