struct WindfoilHeader {
    bounds: vec4<f32>,
    band_count: u32,
    band_height: f32,
    inverse_height: f32,
    padding: u32,
    raw_start: u32,
    raw_count: u32,
    raw_padding: vec2<u32>,
};

@group(1) @binding(0) var<storage, read> path_points: array<vec2<f32>>;
@group(1) @binding(1) var<storage, read> path_rows: array<vec2<u32>>;
@group(1) @binding(2) var<uniform> path_header: WindfoilHeader;

struct WindfoilOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local_position: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) @interpolate(flat) matrix_0: vec4<f32>,
    @location(3) @interpolate(flat) matrix_1: vec4<f32>,
    @location(4) @interpolate(flat) matrix_2: vec4<f32>,
    @location(5) @interpolate(flat) matrix_3: vec4<f32>,
    @location(6) @interpolate(flat) motion: u32,
    @location(7) @interpolate(flat) start_time: u32,
    @location(8) @interpolate(flat) gain: f32,
    @location(9) @interpolate(flat) duration: u32,
    @location(10) @interpolate(flat) projected: u32,
};

fn project_local(point: vec2<f32>, instances: InstancesInput) -> vec4<f32> {
    return transform_vertex(VertexInput(0u, point, 2u), instances).clip_position;
}

@vertex
fn vs_windfoil(@builtin(vertex_index) vertex: u32, instances: InstancesInput) -> WindfoilOutput {
    let corner = vec2<f32>(f32(vertex & 1u), f32(vertex >> 1u));
    var output: WindfoilOutput;
    output.color = instances.color;
    output.matrix_0 = instances.model_matrix_0;
    output.matrix_1 = instances.model_matrix_1;
    output.matrix_2 = instances.model_matrix_2;
    output.matrix_3 = instances.model_matrix_3;
    output.motion = instances.motion;
    output.start_time = instances.start_time;
    output.gain = instances.gain;
    output.duration = instances.duration;
    output.projected = select(0u, 1u, (instances.motion & (7u << 23u)) != 0u && instances.gain != 0.0);
    if output.projected != 0u {
        var lower = vec2<f32>(1e30);
        var upper = vec2<f32>(-1e30);
        var depth = 0.0;
        for (var index = 0u; index < path_header.raw_count * 3u; index += 1u) {
            let projected = project_local(path_points[path_header.raw_start * 3u + index], instances);
            let position = projected.xy / projected.w;
            lower = min(lower, position);
            upper = max(upper, position);
            depth = projected.z / projected.w;
        }
        let padding = 2.0 / vec2<f32>(f32(u_buffer.u_width), f32(u_buffer.u_height));
        output.position = vec4<f32>(mix(lower - padding, upper + padding, corner), depth, 1.0);
        output.local_position = vec2<f32>(0.0);
        return output;
    }
    let point = mix(path_header.bounds.xy, path_header.bounds.zw, corner);
    let center = project_local(point, instances);
    let horizontal = project_local(point + vec2<f32>(1.0, 0.0), instances);
    let vertical = project_local(point + vec2<f32>(0.0, 1.0), instances);
    let dimensions = vec2<f32>(f32(u_buffer.u_width), f32(u_buffer.u_height)) * 0.5;
    let dx = (horizontal.xy / horizontal.w - center.xy / center.w) * dimensions;
    let dy = (vertical.xy / vertical.w - center.xy / center.w) * dimensions;
    let determinant = max(abs(dx.x * dy.y - dx.y * dy.x), 1e-12);
    let padding = vec2<f32>(abs(dy.x) + abs(dy.y), abs(dx.x) + abs(dx.y)) / determinant;
    let local = point + (corner * 2.0 - 1.0) * padding;
    output.position = project_local(local, instances);
    output.local_position = local;
    return output;
}

fn screen_point(point: vec2<f32>, instances: InstancesInput) -> vec2<f32> {
    let clip = project_local(point, instances);
    let normalized = clip.xy / clip.w;
    return (normalized * vec2<f32>(0.5, -0.5) + 0.5) * vec2<f32>(f32(u_buffer.u_width), f32(u_buffer.u_height));
}

fn interval_integral(start: vec2<f32>, control: vec2<f32>, end: vec2<f32>) -> f32 {
    if max(start.x, end.x) <= -0.5 || min(start.y, end.y) >= 0.5 || max(start.y, end.y) <= -0.5 { return 0.0; }
    if min(start.x, end.x) >= 0.5 { return clamp(end.y, -0.5, 0.5) - clamp(start.y, -0.5, 0.5); }
    return piece_integral(start, control, end, -0.5, 0.5, 0.5);
}

fn interval_sample(start: vec2<f32>, control: vec2<f32>, end: vec2<f32>) -> f32 {
    if min(start.y, end.y) > 0.0 || max(start.y, end.y) <= 0.0 { return 0.0; }
    let quadratic = start - 2.0 * control + end;
    let linear = 2.0 * (control - start);
    let parameter = monotone_root(quadratic.y, linear.y, start.y, end.y, 0.0);
    return select(0.0, select(-1.0, 1.0, end.y > start.y), (quadratic.x * parameter + linear.x) * parameter + start.x > 0.0);
}

fn interior_extremum(numerator: f32, denominator: f32) -> f32 {
    if denominator == 0.0 { return 1.0; }
    let parameter = numerator / denominator;
    return select(1.0, parameter, parameter > 0.0 && parameter < 1.0);
}

fn projected_integral(input: WindfoilOutput) -> f32 {
    let instances = InstancesInput(input.matrix_0, input.matrix_1, input.matrix_2, input.matrix_3,
        input.color, input.motion, input.start_time, input.gain, input.duration);
    var area = 0.0;
    for (var index = 0u; index < path_header.raw_count; index += 1u) {
        let base = (path_header.raw_start + index) * 3u;
        let start = screen_point(path_points[base], instances) - input.position.xy;
        let control = screen_point(path_points[base + 1u], instances) - input.position.xy;
        let end = screen_point(path_points[base + 2u], instances) - input.position.xy;
        let denominator = start - 2.0 * control + end;
        let extrema = vec2<f32>(interior_extremum(start.x - control.x, denominator.x),
            interior_extremum(start.y - control.y, denominator.y));
        let first = min(extrema.x, extrema.y);
        let second = max(extrema.x, extrema.y);
        var remaining_start = start;
        var remaining_control = control;
        var previous = 0.0;
        for (var piece = 0u; piece < 3u; piece += 1u) {
            let next = select(select(first, second, piece == 1u), 1.0, piece == 2u);
            if previous >= 1.0 { break; }
            if next <= previous { continue; }
            let parameter = (next - previous) / (1.0 - previous);
            let first_control = mix(remaining_start, remaining_control, parameter);
            let second_control = mix(remaining_control, end, parameter);
            let middle = mix(first_control, second_control, parameter);
            if u_buffer.u_antialiasing != 0u {
                area += interval_integral(remaining_start, first_control, middle);
            } else {
                area += interval_sample(remaining_start, first_control, middle);
            }
            remaining_start = middle;
            remaining_control = second_control;
            previous = next;
        }
    }
    return area;
}

fn monotone_root(quadratic: f32, linear: f32, start: f32, end: f32, value: f32) -> f32 {
    let rising = end >= start;
    if select(start <= value, start >= value, rising) { return 0.0; }
    if select(end >= value, end <= value, rising) { return 1.0; }
    let constant = start - value;
    if abs(quadratic) < 1e-12 * max(abs(linear), 1.0) {
        return clamp(-constant / linear, 0.0, 1.0);
    }
    let radical = sqrt(max(linear * linear - 4.0 * quadratic * constant, 0.0));
    let stable = -0.5 * (linear + select(-radical, radical, linear >= 0.0));
    let first = (linear < 0.0) == rising;
    let numerator = select(constant, stable, first);
    let denominator = select(stable, quadratic, first);
    return clamp(numerator / select(1.0, denominator, denominator != 0.0), 0.0, 1.0);
}

fn inside_integral(quadratic: vec2<f32>, linear: vec2<f32>, start_x: f32, first: f32, last: f32, half_width: f32) -> f32 {
    let middle = (first + last) * 0.5;
    let radius = max(last - first, 0.0) * 0.5;
    let horizontal = (quadratic.x * middle + linear.x) * middle + start_x + half_width;
    let derivative = 2.0 * quadratic * middle + linear;
    return 2.0 * radius * horizontal * derivative.y
        + (2.0 / 3.0) * radius * radius * radius * (quadratic.x * derivative.y + 2.0 * quadratic.y * derivative.x);
}

fn piece_integral(start: vec2<f32>, control: vec2<f32>, end: vec2<f32>, low: f32, high: f32, half_width: f32) -> f32 {
    let quadratic = start - 2.0 * control + end;
    let linear = 2.0 * (control - start);
    let rising_y = end.y >= start.y;
    let first = monotone_root(quadratic.y, linear.y, start.y, end.y, select(high, low, rising_y));
    let last = monotone_root(quadratic.y, linear.y, start.y, end.y, select(low, high, rising_y));
    if last <= first { return 0.0; }
    let left = clamp(monotone_root(quadratic.x, linear.x, start.x, end.x, -half_width), first, last);
    let right = clamp(monotone_root(quadratic.x, linear.x, start.x, end.x, half_width), first, last);
    let rising_x = end.x >= start.x;
    let inside_first = select(right, left, rising_x);
    let inside_last = max(select(left, right, rising_x), inside_first);
    let full_first = select(first, inside_last, rising_x);
    let full_last = select(inside_first, last, rising_x);
    let full_middle = (full_first + full_last) * 0.5;
    return inside_integral(quadratic, linear, start.x, inside_first, inside_last, half_width)
        + max(full_last - full_first, 0.0) * (2.0 * quadratic.y * full_middle + linear.y) * (2.0 * half_width);
}

fn row_index(offset: f32) -> u32 {
    return u32(clamp(floor(offset * path_header.inverse_height), 0.0, f32(path_header.band_count - 1u)));
}

fn winding_integral(center: vec2<f32>, footprint: vec2<f32>) -> f32 {
    let half_size = footprint * 0.5;
    let origin_y = path_header.bounds.y - center.y;
    let first_row = row_index(-origin_y - half_size.y);
    let last_row = row_index(-origin_y + half_size.y);
    var area = 0.0;
    for (var row = first_row; row <= last_row; row += 1u) {
        var low = -half_size.y;
        var high = half_size.y;
        if path_header.band_count > 1u {
            low = max(low, origin_y + f32(row) * path_header.band_height);
            high = min(high, origin_y + f32(row + 1u) * path_header.band_height);
        }
        if high <= low { continue; }
        let entry = path_rows[row];
        for (var index = 0u; index < entry.y; index += 1u) {
            let base = (entry.x + index) * 3u;
            let start = path_points[base] - center;
            let end = path_points[base + 2u] - center;
            if max(start.x, end.x) <= -half_size.x { break; }
            if min(start.y, end.y) >= high || max(start.y, end.y) <= low { continue; }
            if min(start.x, end.x) >= half_size.x {
                area += footprint.x * (clamp(end.y, low, high) - clamp(start.y, low, high));
            } else {
                area += piece_integral(start, path_points[base + 1u] - center, end, low, high, half_size.x);
            }
        }
    }
    return area / (footprint.x * footprint.y);
}

fn winding_sample(center: vec2<f32>) -> f32 {
    let entry = path_rows[row_index(center.y - path_header.bounds.y)];
    var winding = 0.0;
    for (var index = 0u; index < entry.y; index += 1u) {
        let base = (entry.x + index) * 3u;
        let start = path_points[base] - center;
        let end = path_points[base + 2u] - center;
        if max(start.x, end.x) <= 0.0 { break; }
        if min(start.y, end.y) > 0.0 || max(start.y, end.y) <= 0.0 { continue; }
        let control = path_points[base + 1u] - center;
        let quadratic = start - 2.0 * control + end;
        let linear = 2.0 * (control - start);
        let parameter = monotone_root(quadratic.y, linear.y, start.y, end.y, 0.0);
        if (quadratic.x * parameter + linear.x) * parameter + start.x > 0.0 {
            winding += select(-1.0, 1.0, end.y > start.y);
        }
    }
    return winding;
}

fn windfoil_color(input: WindfoilOutput, even_odd: bool) -> vec4<f32> {
    let footprint = max(fwidth(input.local_position), vec2<f32>(1e-9));
    var winding = 0.0;
    if input.projected != 0u {
        winding = projected_integral(input);
    } else if u_buffer.u_antialiasing == 0u {
        winding = winding_sample(input.local_position);
    } else {
        winding = winding_integral(input.local_position, footprint);
    }
    let magnitude = abs(winding);
    let parity = magnitude - 2.0 * floor(magnitude * 0.5);
    let coverage = select(min(magnitude, 1.0), 1.0 - abs(1.0 - parity), even_odd);
    return vec4<f32>(input.color * coverage, coverage);
}

@fragment
fn fs_windfoil_non_zero(input: WindfoilOutput) -> @location(0) vec4<f32> {
    return windfoil_color(input, false);
}

@fragment
fn fs_windfoil_even_odd(input: WindfoilOutput) -> @location(0) vec4<f32> {
    return windfoil_color(input, true);
}