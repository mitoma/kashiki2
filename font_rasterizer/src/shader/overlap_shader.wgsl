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
    let duration: u32 = 500u;
    var v = 0f;
    let mogmog = u_buffer.u_time - instances.start_time;
    if mogmog <= 0u {
        v = 0f;
    } else if mogmog >= duration {
        v = 1f;
    } else {
        v = f32(mogmog) / f32(duration);
    }
    v = easing_function(v, EASING_BOUNCE, true, true);

    let gain = v / 2f;
    //let gain = (sin(v * DOUBLE_PI) * model.position.y * 0.5);
    var x_gain: f32 = 0f;
    var y_gain: f32 = 0f;
    var z_gain: f32 = 0f;
    var x_rotate: f32 = 0f;
    var y_rotate: f32 = 0f;
    var z_rotate: f32 = 0f;

    if bit_check(instances.motion, 0u) {
        x_gain += gain;
    }
    if bit_check(instances.motion, 1u) {
        y_gain += gain;
    }
    if bit_check(instances.motion, 2u) {
        z_gain += gain;
    }
    if bit_check(instances.motion, 3u) {
        x_rotate = 1.0f;
    }
    if bit_check(instances.motion, 4u) {
        y_rotate = 1.0f;
    }
    if bit_check(instances.motion, 5u) {
        z_rotate = 1.0f;
    }

    var moved = vec4<f32>(
        model.position.x + x_gain,
        model.position.y + y_gain,
        0.0 + z_gain,
        1.0
    );

    if x_rotate != 0f || y_rotate != 0f || z_rotate != 0f {
        moved = vec4<f32>(rotate(moved.xyz, sin(v * 3.14 * 2f) * distance(moved.xy, vec2<f32>(0f, 0f)) * 4f, vec3<f32>(x_rotate, y_rotate, z_rotate)), 1.0);
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
