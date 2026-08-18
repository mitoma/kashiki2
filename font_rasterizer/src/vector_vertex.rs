use bezier_converter::CubicBezier;
use log::debug;
use skrifa::outline::OutlinePen;

pub struct VectorVertexBuilder {
    vertex: Vec<InternalVertex>,
    index: Vec<u32>,
    current_index: u32,
    path_start_index: Option<u32>,
    subpath_index_start: usize,
    subpath_points: Vec<[f32; 2]>,
    vertex_swap: FlipFlop,
    builder_options: VertexBuilderOptions,
}

impl Default for VectorVertexBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl VectorVertexBuilder {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            vertex: Vec::new(),
            index: Vec::new(),
            // index 0 は原点B、1 は原点L に予約されているので、2 から開始する
            current_index: 1,
            path_start_index: None,
            subpath_index_start: 0,
            subpath_points: Vec::new(),
            vertex_swap: FlipFlop::Flip,
            builder_options: VertexBuilderOptions::default(),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub(crate) fn with_options(self, builder_options: VertexBuilderOptions) -> Self {
        Self {
            vertex: self.vertex,
            index: self.index,
            current_index: self.current_index,
            path_start_index: self.path_start_index,
            subpath_index_start: self.subpath_index_start,
            subpath_points: self.subpath_points,
            vertex_swap: self.vertex_swap,
            builder_options,
        }
    }

    #[inline]
    fn next_wait(&mut self) -> FlipFlop {
        self.vertex_swap = self.vertex_swap.next();
        self.vertex_swap
    }

    pub fn build(self) -> VectorVertex {
        let center: [f32; 2] = self.builder_options.center;
        let unit_em: f32 = self.builder_options.unit_em;
        let coordinate_system = self.builder_options.coordinate_system;
        let scale_option = self.builder_options.scale;
        let [center_x, center_y] = coordinate_system.transform(center[0], center[1]);
        let [center_x, center_y] = scale_option.map_or([center_x, center_y], |[width, height]| {
            [center_x * width, center_y * height]
        });

        let vertex = self
            .vertex
            .iter()
            .map(|InternalVertex { x, y, wait }| {
                let [x, y] = coordinate_system.transform(*x, *y);
                let [x, y] = [(x - center_x) / unit_em, (y - center_y) / unit_em];
                let [x, y] = scale_option.map_or([x, y], |[width, height]| [x * width, y * height]);
                Vertex {
                    position: [x, y],
                    vertex_type: wait.vertex_type(),
                }
            })
            .collect();
        VectorVertex {
            vertex,
            index: self.index,
        }
    }

    pub fn move_to(&mut self, x: f32, y: f32) {
        let wait = self.next_wait();
        self.subpath_index_start = self.index.len();
        self.subpath_points.clear();
        self.subpath_points.push([x, y]);
        self.vertex.push(InternalVertex { x, y, wait });
        self.vertex.push(InternalVertex {
            x,
            y,
            wait: wait.for_line(),
        });
        self.path_start_index = Some(self.current_index);
        self.current_index += 2;
    }

    pub fn line_to(&mut self, x: f32, y: f32) {
        let Some(last) = &self.vertex.last() else {
            return;
        };
        if last.x == x && last.y == y {
            // 同じ座標への line_to は無視する
            return;
        }
        self.subpath_points.push([x, y]);

        let wait = self.next_wait();
        self.vertex.push(InternalVertex { x, y, wait });
        self.vertex.push(InternalVertex {
            x,
            y,
            wait: wait.for_line(),
        });
        self.index.push(1); // 原点L の index
        self.index.push(self.current_index);
        self.index.push(self.current_index + 2);
        self.current_index += 2;
    }

    pub fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let Some(last) = &self.vertex.last() else {
            return;
        };
        if last.x == x1 && last.y == y1 && last.x == x && last.y == y {
            return;
        }
        // ベジエ補助直線（フィル）三角形専用頂点のために、直前のオンカーブ点座標を保持する
        let prev_x = last.x;
        let prev_y = last.y;

        let wait = self.next_wait();
        self.subpath_points.push([x, y]);

        // quad_to 開始時点の current_index。以降 push する頂点の index 値は ci + 1 + k（k は push 順）。
        let ci = self.current_index;

        // ベジエ曲線・ベジエ補助直線・直線の三種の三角形が頂点を共用すると
        // シェーダー側で triangle_type が混在して区別できないため、
        // 補助直線（フィル）三角形には専用頂点を割り当てて頂点を共用しないようにする。

        // k0: 制御点（ベジエ曲線三角形用）
        self.vertex.push(InternalVertex {
            x: x1,
            y: y1,
            wait: FlipFlop::Control,
        });
        // k1: ベジエ補助直線 終点（この区間のオンカーブ終点座標）
        self.vertex.push(InternalVertex {
            x,
            y,
            wait: FlipFlop::BezierFillEnd,
        });
        // k2: ベジエ補助直線 始点（直前のオンカーブ点座標）
        self.vertex.push(InternalVertex {
            x: prev_x,
            y: prev_y,
            wait: FlipFlop::BezierFillStart,
        });
        // k3: 終点B（ベジエ曲線三角形用。次区間の prev endpoint として参照される）
        self.vertex.push(InternalVertex { x, y, wait });
        // k4: 終点L（直線三角形用）
        self.vertex.push(InternalVertex {
            x,
            y,
            wait: wait.for_line(),
        });

        // ベジエ補助直線（フィル）三角形: [原点B, 補助直線始点(k2), 補助直線終点(k1)]
        self.index.push(0); // 原点B の index
        self.index.push(ci + 3); // 補助直線始点 (k2)
        self.index.push(ci + 2); // 補助直線終点 (k1)

        // ベジエ曲線三角形: [直前の終点B, 制御点(k0), この区間の終点B(k3)]
        self.index.push(ci - 1); // 直前の終点B
        self.index.push(ci + 1); // 制御点 (k0)
        self.index.push(ci + 4); // この区間の終点B (k3)
        self.current_index += 5;
    }

    pub fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        // 3 次ベジエを 2 次ベジエに近似する
        let last = &self.vertex.last().unwrap();
        if last.x == x1
            && last.y == y1
            && last.x == x2
            && last.y == y2
            && last.x == x
            && last.y == y
        {
            return;
        }

        let cb = CubicBezier {
            x0: last.x,
            y0: last.y,
            x1: x,
            y1: y,
            cx0: x1,
            cy0: y1,
            cx1: x2,
            cy1: y2,
        };
        let qbs = cb.to_quadratic();
        debug!("cubic to quadratic: 1 -> {}", qbs.len());
        for qb in qbs.iter() {
            self.quad_to(qb.cx0, qb.cy0, qb.x1, qb.y1)
        }
    }

    pub fn close(&mut self) {
        if let Some(start_index) = self.path_start_index {
            let start_vertex = &self.vertex[(start_index) as usize];
            self.line_to(start_vertex.x, start_vertex.y);

            // close されたサブパスごとに重心原点を 2 つ（Bezier/Line）追加する
            // 0/1 は global zero vertex だが、ここでサブパス専用原点へ置換する
            if !self.subpath_points.is_empty() {
                let [centroid_x, centroid_y] = calculate_subpath_center(
                    &self.subpath_points,
                    self.builder_options.center_point_algorithm,
                );

                let bezier_origin_index = self.current_index + 1;
                let line_origin_index = self.current_index + 2;
                self.vertex.push(InternalVertex {
                    x: centroid_x,
                    y: centroid_y,
                    wait: FlipFlop::OriginBezier,
                });
                self.vertex.push(InternalVertex {
                    x: centroid_x,
                    y: centroid_y,
                    wait: FlipFlop::OriginLine,
                });
                self.current_index += 2;

                for idx in &mut self.index[self.subpath_index_start..] {
                    if *idx == 0 {
                        *idx = bezier_origin_index;
                    } else if *idx == 1 {
                        *idx = line_origin_index;
                    }
                }
            }

            self.path_start_index = None;
        }
    }
}

impl OutlinePen for VectorVertexBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.line_to(x, y);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.quad_to(cx0, cy0, x, y);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.curve_to(cx0, cy0, cx1, cy1, x, y);
    }

    fn close(&mut self) {
        self.close();
    }
}

pub enum CoordinateSystem {
    Svg,  // SVGの座標系 (左上原点, Y軸が下方向)
    Font, // フォント座標系 (ベースライン原点, Y軸が上方向)
}

impl CoordinateSystem {
    #[inline]
    pub(crate) fn transform(&self, x: f32, y: f32) -> [f32; 2] {
        match self {
            CoordinateSystem::Svg => [x, -y],
            CoordinateSystem::Font => [x, y],
        }
    }
}

pub(crate) struct VertexBuilderOptions {
    pub(crate) center: [f32; 2],
    pub(crate) unit_em: f32,
    pub(crate) coordinate_system: CoordinateSystem,
    pub(crate) scale: Option<[f32; 2]>,
    pub(crate) center_point_algorithm: CenterPointAlgorithm,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum CenterPointAlgorithm {
    ArithmeticMean,
    MinimizeMaximumAngle,
    MaximizeMinimumAngle,
}

impl Default for VertexBuilderOptions {
    fn default() -> Self {
        Self {
            center: [0.0, 0.0],
            unit_em: 1.0,
            coordinate_system: CoordinateSystem::Font,
            scale: None,
            center_point_algorithm: CenterPointAlgorithm::MaximizeMinimumAngle,
        }
    }
}

impl VertexBuilderOptions {
    pub fn new(
        center: [f32; 2],
        unit_em: f32,
        coordinate_system: CoordinateSystem,
        scale: Option<[f32; 2]>,
    ) -> Self {
        Self {
            center,
            unit_em,
            coordinate_system,
            scale,
            center_point_algorithm: CenterPointAlgorithm::MaximizeMinimumAngle,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn with_center_point_algorithm(
        mut self,
        center_point_algorithm: CenterPointAlgorithm,
    ) -> Self {
        self.center_point_algorithm = center_point_algorithm;
        self
    }
}

fn calculate_subpath_center(points: &[[f32; 2]], algorithm: CenterPointAlgorithm) -> [f32; 2] {
    log::info!("calculate_subpath_center: algorithm = {:?}", algorithm);

    let n = points.len() as f32;
    let arithmetic_mean = [
        points.iter().map(|p| p[0]).sum::<f32>() / n,
        points.iter().map(|p| p[1]).sum::<f32>() / n,
    ];

    match algorithm {
        CenterPointAlgorithm::ArithmeticMean => arithmetic_mean,
        CenterPointAlgorithm::MinimizeMaximumAngle => {
            minimize_maximum_angle(points, arithmetic_mean)
        }
        CenterPointAlgorithm::MaximizeMinimumAngle => {
            maximize_minimum_angle(points, arithmetic_mean)
        }
    }
}

fn minimize_maximum_angle(points: &[[f32; 2]], initial: [f32; 2]) -> [f32; 2] {
    log::info!("minimize_maximum_angle: initial = {:?}", initial);
    if points.len() < 3 {
        log::info!("minimize_maximum_angle: points.len() < 3, returning initial");
        return initial;
    }

    let mut min = points[0];
    let mut max = points[0];
    for &[x, y] in &points[1..] {
        min[0] = min[0].min(x);
        min[1] = min[1].min(y);
        max[0] = max[0].max(x);
        max[1] = max[1].max(y);
    }

    let mut center = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
    let mut step = (max[0] - min[0]).max(max[1] - min[1]) * 0.5;
    let mut best_angle = maximum_subpath_angle(points, center);
    let initial_angle = maximum_subpath_angle(points, initial);
    if initial_angle < best_angle {
        center = initial;
        best_angle = initial_angle;
    }

    for _ in 0..10 {
        let previous_center = center;
        for y in -1..=1 {
            for x in -1..=1 {
                let candidate = [
                    previous_center[0] + x as f32 * step,
                    previous_center[1] + y as f32 * step,
                ];
                let angle = maximum_subpath_angle(points, candidate);
                if angle < best_angle {
                    center = candidate;
                    best_angle = angle;
                }
            }
        }
        step *= 0.5;
    }

    log::info!("minimize_maximum_angle: center = {:?}", center);
    center
}

fn maximum_subpath_angle(points: &[[f32; 2]], center: [f32; 2]) -> f32 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .filter_map(|(start, end)| {
            let edge = [end[0] - start[0], end[1] - start[1]];
            if edge[0] * edge[0] + edge[1] * edge[1] <= f32::EPSILON {
                return None;
            }
            let a = [start[0] - center[0], start[1] - center[1]];
            let b = [end[0] - center[0], end[1] - center[1]];
            let cross = a[0] * b[1] - a[1] * b[0];
            let dot = a[0] * b[0] + a[1] * b[1];
            Some(cross.abs().atan2(dot))
        })
        .fold(0.0, f32::max)
}

fn maximize_minimum_angle(points: &[[f32; 2]], initial: [f32; 2]) -> [f32; 2] {
    if points.len() < 3 {
        return initial;
    }

    let mut min = points[0];
    let mut max = points[0];
    for &[x, y] in &points[1..] {
        min[0] = min[0].min(x);
        min[1] = min[1].min(y);
        max[0] = max[0].max(x);
        max[1] = max[1].max(y);
    }

    let mut center = initial;
    let mut best_angle = minimum_subpath_angle(points, center);
    const GRID_SIZE: usize = 17;
    let span = [max[0] - min[0], max[1] - min[1]];

    // 外接矩形全体を走査して、初期中心付近の局所解に依存しないようにする。
    for y in 0..GRID_SIZE {
        for x in 0..GRID_SIZE {
            let candidate = [
                min[0] + span[0] * x as f32 / (GRID_SIZE - 1) as f32,
                min[1] + span[1] * y as f32 / (GRID_SIZE - 1) as f32,
            ];
            let angle = minimum_subpath_angle(points, candidate);
            if angle > best_angle {
                center = candidate;
                best_angle = angle;
            }
        }
    }

    let mut step = span[0].max(span[1]) / (GRID_SIZE - 1) as f32;
    for _ in 0..8 {
        let previous_center = center;
        for y in -1..=1 {
            for x in -1..=1 {
                let candidate = [
                    (previous_center[0] + x as f32 * step).clamp(min[0], max[0]),
                    (previous_center[1] + y as f32 * step).clamp(min[1], max[1]),
                ];
                let angle = minimum_subpath_angle(points, candidate);
                println!(
                    "candidate: {:?}, angle: {}, best_angle: {}",
                    candidate, angle, best_angle
                );
                if angle > best_angle {
                    center = candidate;
                    best_angle = angle;
                }
            }
        }
        step *= 0.5;
    }

    center
}

fn minimum_subpath_angle(points: &[[f32; 2]], center: [f32; 2]) -> f32 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .filter_map(|(start, end)| {
            let edge = [end[0] - start[0], end[1] - start[1]];
            if edge[0] * edge[0] + edge[1] * edge[1] <= f32::EPSILON {
                return None;
            }
            let a = [start[0] - center[0], start[1] - center[1]];
            let b = [end[0] - center[0], end[1] - center[1]];
            let cross = a[0] * b[1] - a[1] * b[0];
            let dot = a[0] * b[0] + a[1] * b[1];
            let angle = cross.abs().atan2(dot);
            ((a[0] * a[0] + a[1] * a[1] > f32::EPSILON)
                && (b[0] * b[0] + b[1] * b[1] > f32::EPSILON))
                .then_some(angle)
        })
        .fold(f32::INFINITY, f32::min)
}

#[derive(Debug)]
pub struct VectorVertex {
    pub(crate) vertex: Vec<Vertex>,
    pub(crate) index: Vec<u32>,
}
impl VectorVertex {
    pub fn vertex_size(&self) -> u64 {
        (self.vertex.len() * std::mem::size_of::<Vertex>()) as u64
    }

    pub fn index_size(&self) -> u64 {
        (self.index.len() * std::mem::size_of::<u32>()) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimize_maximum_angle_keeps_the_worst_angle_small() {
        let points = [[0.0, 0.0], [10.0, 0.0], [8.0, 1.0], [0.0, 4.0]];
        let arithmetic_mean =
            calculate_subpath_center(&points, CenterPointAlgorithm::ArithmeticMean);
        let optimized =
            calculate_subpath_center(&points, CenterPointAlgorithm::MinimizeMaximumAngle);

        assert!(
            maximum_subpath_angle(&points, optimized)
                <= maximum_subpath_angle(&points, arithmetic_mean)
        );
    }

    #[test]
    fn maximize_minimum_angle_keeps_the_smallest_angle_large() {
        let points = [[0.0, 0.0], [10.0, 0.0], [8.0, 1.0], [0.0, 4.0]];
        let arithmetic_mean =
            calculate_subpath_center(&points, CenterPointAlgorithm::ArithmeticMean);
        let optimized =
            calculate_subpath_center(&points, CenterPointAlgorithm::MaximizeMinimumAngle);

        assert!(
            minimum_subpath_angle(&points, optimized)
                >= minimum_subpath_angle(&points, arithmetic_mean)
        );
    }

    #[test]
    fn minimum_angle_ignores_duplicate_closing_point() {
        let closed_points = [
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 10.0],
            [0.0, 10.0],
            [0.0, 0.0],
        ];

        assert!(minimum_subpath_angle(&closed_points, [5.0, 5.0]) > 0.0);
    }

    #[test]
    fn maximum_angle_ignores_duplicate_closing_point() {
        let points = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let closed_points = [
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 10.0],
            [0.0, 10.0],
            [0.0, 0.0],
        ];

        assert_eq!(
            maximum_subpath_angle(&points, [5.0, 5.0]),
            maximum_subpath_angle(&closed_points, [5.0, 5.0])
        );
    }

    #[test]
    fn center_point_algorithm_is_applied_when_closing() {
        let points = [[0.0, 0.0], [10.0, 0.0], [8.0, 1.0], [0.0, 4.0]];
        let closed_points = [[0.0, 0.0], [10.0, 0.0], [8.0, 1.0], [0.0, 4.0], [0.0, 0.0]];
        let mut builder = VectorVertexBuilder::new().with_options(
            VertexBuilderOptions::default()
                .with_center_point_algorithm(CenterPointAlgorithm::MinimizeMaximumAngle),
        );
        builder.move_to(points[0][0], points[0][1]);
        for point in &points[1..] {
            builder.line_to(point[0], point[1]);
        }
        builder.close();

        let origin = builder
            .vertex
            .iter()
            .find(|vertex| matches!(vertex.wait, FlipFlop::OriginLine))
            .unwrap();
        let expected =
            calculate_subpath_center(&closed_points, CenterPointAlgorithm::MinimizeMaximumAngle);
        assert_eq!([origin.x, origin.y], expected);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Vertex {
    pub(crate) position: [f32; 2],
    pub(crate) vertex_type: u32,
}

impl Vertex {
    pub(crate) fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // 文字情報なので xy の座標だけでよい
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum FlipFlop {
    Flip,
    Flop,
    Control,
    FlipForLine,
    FlopForLine,
    OriginBezier,
    OriginLine,
    // ベジエ補助直線（フィル）三角形専用の頂点。ベジエ曲線三角形と頂点を共用しない。
    BezierFillStart,
    BezierFillEnd,
}

impl FlipFlop {
    #[inline]
    pub(crate) fn next(&self) -> Self {
        match self {
            FlipFlop::Flip => FlipFlop::Flop,
            FlipFlop::Flop => FlipFlop::Flip,
            FlipFlop::Control => FlipFlop::Control,
            FlipFlop::FlipForLine => FlipFlop::FlipForLine,
            FlipFlop::FlopForLine => FlipFlop::FlopForLine,
            FlipFlop::OriginBezier => FlipFlop::OriginBezier,
            FlipFlop::OriginLine => FlipFlop::OriginLine,
            FlipFlop::BezierFillStart => FlipFlop::BezierFillStart,
            FlipFlop::BezierFillEnd => FlipFlop::BezierFillEnd,
        }
    }

    pub(crate) fn for_line(&self) -> Self {
        match self {
            FlipFlop::Flip => FlipFlop::FlopForLine,
            FlipFlop::Flop => FlipFlop::FlipForLine,
            FlipFlop::Control => FlipFlop::Control,
            FlipFlop::FlipForLine => FlipFlop::FlipForLine,
            FlipFlop::FlopForLine => FlipFlop::FlopForLine,
            FlipFlop::OriginBezier => FlipFlop::OriginBezier,
            FlipFlop::OriginLine => FlipFlop::OriginLine,
            FlipFlop::BezierFillStart => FlipFlop::BezierFillStart,
            FlipFlop::BezierFillEnd => FlipFlop::BezierFillEnd,
        }
    }

    #[inline]
    pub(crate) fn vertex_type(&self) -> u32 {
        match self {
            FlipFlop::Flip => 2,
            FlipFlop::FlipForLine => 3,
            FlipFlop::Flop => 4,
            FlipFlop::FlopForLine => 5,
            FlipFlop::Control => 6,
            FlipFlop::OriginBezier => 0,
            FlipFlop::OriginLine => 1,
            FlipFlop::BezierFillStart => 7,
            FlipFlop::BezierFillEnd => 8,
        }
    }
}

pub(crate) struct InternalVertex {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) wait: FlipFlop,
}
