use bezier_converter::CubicBezier;
use glam::Vec2;
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
    // 現在のサブパスの開始点。直線ラン簡約時の基準点として使う
    subpath_start: [f32; 2],
    // 直前にバッファへ積んだ点（未確定の論理上の「現在位置」）
    last_pending_point: [f32; 2],
    // move_to 〜 close の間に溜める、簡約前の生コマンド列
    pending_segments: Vec<PendingSegment>,
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
            subpath_start: [0.0, 0.0],
            last_pending_point: [0.0, 0.0],
            pending_segments: Vec::new(),
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
            subpath_start: self.subpath_start,
            last_pending_point: self.last_pending_point,
            pending_segments: self.pending_segments,
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

    /// move_to() が呼ばれた実体。頂点を即座に生成する。
    fn real_move_to(&mut self, x: f32, y: f32) {
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

    /// line_to() が呼ばれた実体。頂点を即座に生成する。
    fn real_line_to(&mut self, x: f32, y: f32) {
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

    /// quad_to() が呼ばれた実体。頂点を即座に生成する。
    fn real_quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let Some(last) = &self.vertex.last() else {
            return;
        };
        if last.x == x1 && last.y == y1 && last.x == x && last.y == y {
            return;
        }
        // ベジエ補助直線（フィル）三角形専用頂点のために、直前のオンカーブ点座標を保持する
        let prev_x = last.x;
        let prev_y = last.y;

        if is_nearly_straight_default([prev_x, prev_y].into(), [x1, y1].into(), [x, y].into()) {
            return self.real_line_to(x, y);
        }

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
        let [last_x, last_y] = self.last_pending_point;
        if last_x == x1
            && last_y == y1
            && last_x == x2
            && last_y == y2
            && last_x == x
            && last_y == y
        {
            return;
        }

        let cb = CubicBezier {
            x0: last_x,
            y0: last_y,
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

    /// フォントによっては、実質的に直線であるパスを複数のベジエ曲線に分割して
    /// 表現していることがある。そのため move_to 〜 close の間はコマンドを
    /// そのままバッファし、close() 時にまとめて直線ランを簡約してから
    /// 実際の頂点を生成する。
    pub fn move_to(&mut self, x: f32, y: f32) {
        if self.path_start_index.is_some() {
            // close() されずに次の move_to が呼ばれた場合の保険。
            // バッファを素通しでそのまま実頂点化する。
            self.flush_pending_segments_raw();
        }
        self.real_move_to(x, y);
        self.subpath_start = [x, y];
        self.last_pending_point = [x, y];
        self.pending_segments.clear();
    }

    pub fn line_to(&mut self, x: f32, y: f32) {
        let [px, py] = self.last_pending_point;
        if px == x && py == y {
            return;
        }
        self.pending_segments.push(PendingSegment::Line { x, y });
        self.last_pending_point = [x, y];
    }

    pub fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let [px, py] = self.last_pending_point;
        if px == x1 && py == y1 && px == x && py == y {
            return;
        }
        self.pending_segments
            .push(PendingSegment::Quad { x1, y1, x, y });
        self.last_pending_point = [x, y];
    }

    fn flush_pending_segments_raw(&mut self) {
        for seg in std::mem::take(&mut self.pending_segments) {
            match seg {
                PendingSegment::Line { x, y } => self.real_line_to(x, y),
                PendingSegment::Quad { x1, y1, x, y } => self.real_quad_to(x1, y1, x, y),
            }
        }
    }

    pub fn close(&mut self) {
        if self.path_start_index.is_none() {
            return;
        }

        // 終点から始点へ戻る閉じ線も簡約対象に含める
        let [start_x, start_y] = self.subpath_start;
        self.line_to(start_x, start_y);

        let segments = std::mem::take(&mut self.pending_segments);
        let simplified = simplify_straight_runs(self.subpath_start.into(), segments);
        for seg in simplified {
            match seg {
                PendingSegment::Line { x, y } => self.real_line_to(x, y),
                PendingSegment::Quad { x1, y1, x, y } => self.real_quad_to(x1, y1, x, y),
            }
        }

        self.real_close();
    }

    /// close() が呼ばれた実体。重心原点の追加など、頂点確定後の後処理を行う。
    fn real_close(&mut self) {
        if self.path_start_index.is_some() {
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

const STRAIGHT_THRESHOLD: f32 = 0.0001;
/// 2次ベジエの1階微分 B'(t)
fn bezier_derivative(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> Vec2 {
    let a = (p1 - p0) * (2.0 * (1.0 - t));
    let b = (p2 - p1) * (2.0 * t);
    a + b
}

/// 2次ベジエの2階微分 B''(t)（t に依存しない）
fn bezier_second_derivative(p0: Vec2, p1: Vec2, p2: Vec2) -> Vec2 {
    (p2 - p1 * 2.0 + p0) * 2.0
}

/// 曲率 κ(t)
fn curvature(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> f32 {
    let d1 = bezier_derivative(p0, p1, p2, t);
    let d2 = bezier_second_derivative(p0, p1, p2);

    // 外積（2D の場合はスカラー）
    let numerator = (d1.x * d2.y - d1.y * d2.x).abs();

    let denom = (d1.length_squared()).powf(1.5);

    if denom < 1e-6 { 0.0 } else { numerator / denom }
}

/// ベジエが「ほぼ直線」かどうか判定する
/// threshold は用途に応じて調整（例: 0.001）
fn is_nearly_straight(p0: Vec2, p1: Vec2, p2: Vec2, threshold: f32) -> bool {
    let mut max_k = 0.0;

    // t をサンプリングして最大曲率を取る
    for i in 0..20 {
        let t = i as f32 / 19.0;
        let k = curvature(p0, p1, p2, t);
        if k > max_k {
            max_k = k;
        }
    }

    max_k < threshold
}

fn is_nearly_straight_default(p0: Vec2, p1: Vec2, p2: Vec2) -> bool {
    is_nearly_straight(p0, p1, p2, STRAIGHT_THRESHOLD)
}

/// move_to 〜 close の間にバッファされる、簡約前の生パスコマンド
enum PendingSegment {
    Line { x: f32, y: f32 },
    Quad { x1: f32, y1: f32, x: f32, y: f32 },
}

// 直線ランの許容誤差。区間長に対する比率としきい値の下限を組み合わせて判定する。
const COLLINEAR_RATIO: f32 = 0.001;
const COLLINEAR_MIN: f32 = 0.01;

/// 点 point と直線 run_start-run_end との距離
fn distance_from_line(run_start: Vec2, run_end: Vec2, point: Vec2) -> f32 {
    let dir = run_end - run_start;
    let len = dir.length();
    if len < 1e-6 {
        return (point - run_start).length();
    }
    let to_point = point - run_start;
    (dir.x * to_point.y - dir.y * to_point.x).abs() / len
}

/// run_start から candidate_end への直線に、points が十分近いか判定する
fn is_collinear(run_start: Vec2, candidate_end: Vec2, points: &[Vec2]) -> bool {
    let len = (candidate_end - run_start).length();
    let threshold = (len * COLLINEAR_RATIO).max(COLLINEAR_MIN);
    points
        .iter()
        .all(|&p| distance_from_line(run_start, candidate_end, p) <= threshold)
}

/// フォントによっては、実質的に直線であるパスを複数の直線・ベジエ曲線に
/// 分割して表現していることがある。単発の quad_to だけでは判定できないため、
/// サブパス単位で溜めたコマンド列を先頭から走査し、連続する「単体でもほぼ直線」な
/// 区間が全体として直線とみなせる場合はまとめて 1 本の line に簡約する。
fn simplify_straight_runs(
    subpath_start: Vec2,
    segments: Vec<PendingSegment>,
) -> Vec<PendingSegment> {
    let mut result = Vec::with_capacity(segments.len());
    let mut run_start = subpath_start;
    let mut run_points: Vec<Vec2> = Vec::new();
    let mut prev = subpath_start;

    for seg in segments {
        let (end, own_straight) = match seg {
            PendingSegment::Line { x, y } => (Vec2::new(x, y), true),
            PendingSegment::Quad { x1, y1, x, y } => {
                let end = Vec2::new(x, y);
                let straight = is_nearly_straight_default(prev, Vec2::new(x1, y1), end);
                (end, straight)
            }
        };

        if own_straight {
            let mut candidate_points = run_points.clone();
            candidate_points.push(prev);
            if is_collinear(run_start, end, &candidate_points) {
                run_points.push(end);
                prev = end;
                continue;
            }
        }

        // ここまでの直線ランを 1 本の直線として確定する
        if let Some(&last) = run_points.last() {
            result.push(PendingSegment::Line {
                x: last.x,
                y: last.y,
            });
            run_start = last;
            run_points.clear();
        }

        if own_straight {
            run_points.push(end);
        } else {
            result.push(seg);
            run_start = end;
        }
        prev = end;
    }

    if let Some(&last) = run_points.last() {
        result.push(PendingSegment::Line {
            x: last.x,
            y: last.y,
        });
    }

    result
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

    #[test]
    fn simplify_straight_runs_merges_collinear_lines() {
        let segments = vec![
            PendingSegment::Line { x: 5.0, y: 0.0 },
            PendingSegment::Line { x: 10.0, y: 0.0 },
            PendingSegment::Line { x: 10.0, y: 10.0 },
        ];
        let simplified = simplify_straight_runs(Vec2::new(0.0, 0.0), segments);

        // 最初の 2 区間（0,0)->(5,0)->(10,0) は同一直線上なので 1 本に簡約される
        assert_eq!(simplified.len(), 2);
        match simplified[0] {
            PendingSegment::Line { x, y } => {
                assert_eq!((x, y), (10.0, 0.0));
            }
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn simplify_straight_runs_merges_nearly_straight_quads() {
        // 制御点が両端点を結ぶ直線上にあるため、曲率 0 の「見せかけの曲線」になる
        let segments = vec![
            PendingSegment::Quad {
                x1: 2.5,
                y1: 0.0,
                x: 5.0,
                y: 0.0,
            },
            PendingSegment::Quad {
                x1: 7.5,
                y1: 0.0,
                x: 10.0,
                y: 0.0,
            },
        ];
        let simplified = simplify_straight_runs(Vec2::new(0.0, 0.0), segments);

        assert_eq!(simplified.len(), 1);
        match simplified[0] {
            PendingSegment::Line { x, y } => assert_eq!((x, y), (10.0, 0.0)),
            _ => panic!("expected the two quads to collapse into a single line"),
        }
    }

    #[test]
    fn simplify_straight_runs_keeps_real_curves() {
        let segments = vec![PendingSegment::Quad {
            x1: 5.0,
            y1: 10.0,
            x: 10.0,
            y: 0.0,
        }];
        let simplified = simplify_straight_runs(Vec2::new(0.0, 0.0), segments);

        assert_eq!(simplified.len(), 1);
        assert!(matches!(simplified[0], PendingSegment::Quad { .. }));
    }

    #[test]
    fn builder_collapses_straight_quad_run_into_single_line_triangle() {
        let mut builder = VectorVertexBuilder::new();
        builder.move_to(0.0, 0.0);
        builder.quad_to(2.5, 0.0, 5.0, 0.0);
        builder.quad_to(7.5, 0.0, 10.0, 0.0);
        builder.line_to(10.0, 10.0);
        builder.close();

        // 直線とみなせる quad_to が連続しているので、ベジエ曲線用の Control 頂点は生成されない
        assert!(
            !builder
                .vertex
                .iter()
                .any(|v| matches!(v.wait, FlipFlop::Control))
        );
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
