// ここはシェーダーで使う便利関数を書くスペース
let PI: f32 = 3.14159265359;
let HALF_PI: f32 = 1.57079632679;
let DOUBLE_PI: f32 = 6.28318530718;

let EASING_LINER: u32 = 0u;
let EASING_SIN: u32 = 1u;
let EASING_QUAD: u32 = 2u;
let EASING_CUBIC: u32 = 3u;
let EASING_QUART: u32 = 4u;
let EASING_QUINT: u32 = 5u;
let EASING_EXPO: u32 = 6u;
let EASING_CIRC: u32 = 7u;
let EASING_BACK: u32 = 8u;
let EASING_ELASTIC: u32 = 9u;
let EASING_BOUNCE: u32 = 10u;

let BOUNCE_N1: f32 = 7.5625;
let BOUNCE_D1: f32 = 2.75;
let BACK_C1: f32 = 1.70158;
let BACK_C3: f32 = 2.70158;
let EXPO: f32 = 2.09439510239;

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
            let value = (value - 0.5f) * 2f;
            let value = 1f - value;
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
    u_time: u32,
};

@group(0) @binding(0)
var<uniform> u_buffer: Uniforms;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec2<f32>,
    @location(1) wait: vec2<f32>,
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
    @location(0) wait: vec3<f32>,
    @location(1) color: vec3<f32>,
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
    let use_distance = bit_check(motion, 26u);

    let easing_type = bit_range(motion, 19u, 16u);
    let duration = instances.duration;
    let gain = instances.gain;
    let distance = distance(model.position.xy, vec2(0f, 0f));

    var v = 0f;
    var easing_position = u_buffer.u_time - instances.start_time;
    if is_loop {
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

    if use_distance {
        calced_gain = calced_gain * distance;
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
    out.wait = vec3<f32>(1f, model.wait.xy);
    out.color = instances.color;
    out.clip_position = u_buffer.u_view_proj * instance_matrix * moved;
    return out;
}

let UNIT :f32 = 0.00390625;
// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // R = ポリゴンの重なった数
    // G, B = ベジエ曲線

    // B が 0 の時はベジエ曲線ではなくて単なるポリゴンとして処理する
    //if in.color.b == 0.0 {
        //return vec4<f32>(in.color, UNIT);
    //}

    // G, B のいずれかが 0 でないとき
    let in_bezier = pow((in.wait.g / 2.0 + in.wait.b), 2.0) < in.wait.b;
    if !in_bezier {
        return vec4<f32>(in.color, 0.0);
    }
    return vec4<f32>(in.color, UNIT);
}
