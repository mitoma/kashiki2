use tiny_skia_path::{NormalizedF32Exclusive, Point, path_geometry};

/// セグメントに共通の操作を定義する trait
pub(crate) trait SegmentTrait: Sized + Clone {
    fn endpoints(&self) -> (Point, Point);
    fn set_from(&mut self, point: Point);
    fn set_to(&mut self, point: Point);
    fn chop(&self, t: f32) -> (Self, Self);
    fn reverse(&self) -> Self;
    fn evaluate(&self, t: f32) -> Point;
    fn flatten(&self, tolerance: f32) -> Vec<Point>;
    /// 始点側の接線ベクトル
    #[allow(clippy::wrong_self_convention)]
    fn from_vector(&self) -> Point;
    /// 終点側の接線ベクトル
    fn to_vector(&self) -> Point;
    fn is_degenerate(&self) -> bool;
    #[allow(dead_code)]
    fn bounding_rect(&self) -> (f32, f32, f32, f32);
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Line {
    pub(crate) from: Point,
    pub(crate) to: Point,
}

impl SegmentTrait for Line {
    fn endpoints(&self) -> (Point, Point) {
        (self.from, self.to)
    }

    fn set_from(&mut self, point: Point) {
        self.from = point;
    }

    fn set_to(&mut self, point: Point) {
        self.to = point;
    }

    fn chop(&self, t: f32) -> (Line, Line) {
        let mid = Point::from_xy(
            self.from.x + t * (self.to.x - self.from.x),
            self.from.y + t * (self.to.y - self.from.y),
        );
        (
            Line {
                from: self.from,
                to: mid,
            },
            Line {
                from: mid,
                to: self.to,
            },
        )
    }

    fn reverse(&self) -> Self {
        Line {
            from: self.to,
            to: self.from,
        }
    }

    fn evaluate(&self, t: f32) -> Point {
        Point::from_xy(
            self.from.x + t * (self.to.x - self.from.x),
            self.from.y + t * (self.to.y - self.from.y),
        )
    }

    fn flatten(&self, _tolerance: f32) -> Vec<Point> {
        vec![self.from, self.to]
    }

    fn from_vector(&self) -> Point {
        self.to - self.from
    }

    fn to_vector(&self) -> Point {
        self.to - self.from
    }

    fn is_degenerate(&self) -> bool {
        let d = self.to - self.from;
        d.x.abs() < 1e-6 && d.y.abs() < 1e-6
    }

    fn bounding_rect(&self) -> (f32, f32, f32, f32) {
        (
            self.from.x.min(self.to.x),
            self.from.y.min(self.to.y),
            self.from.x.max(self.to.x),
            self.from.y.max(self.to.y),
        )
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Quadratic {
    pub(crate) from: Point,
    pub(crate) to: Point,
    pub(crate) control: Point,
}

impl SegmentTrait for Quadratic {
    fn endpoints(&self) -> (Point, Point) {
        (self.from, self.to)
    }

    fn set_from(&mut self, point: Point) {
        self.from = point;
    }

    fn set_to(&mut self, point: Point) {
        self.to = point;
    }

    fn chop(&self, t: f32) -> (Quadratic, Quadratic) {
        let mut result = [Point::default(); 5];
        let center = NormalizedF32Exclusive::new_bounded(t);
        let arg = [self.from, self.control, self.to];
        path_geometry::chop_quad_at(&arg, center, &mut result);
        (
            Quadratic {
                from: result[0],
                control: result[1],
                to: result[2],
            },
            Quadratic {
                from: result[2],
                control: result[3],
                to: result[4],
            },
        )
    }

    fn reverse(&self) -> Self {
        Quadratic {
            from: self.to,
            to: self.from,
            control: self.control,
        }
    }

    fn evaluate(&self, t: f32) -> Point {
        let mt = 1.0 - t;
        let x = mt * mt * self.from.x + 2.0 * mt * t * self.control.x + t * t * self.to.x;
        let y = mt * mt * self.from.y + 2.0 * mt * t * self.control.y + t * t * self.to.y;
        Point::from_xy(x, y)
    }

    fn flatten(&self, tolerance: f32) -> Vec<Point> {
        let mut points = vec![self.from];
        flatten_quadratic(self.from, self.control, self.to, tolerance, &mut points);
        points
    }

    fn from_vector(&self) -> Point {
        let v = self.control - self.from;
        if v.x.abs() < 1e-10 && v.y.abs() < 1e-10 {
            self.to - self.from
        } else {
            v
        }
    }

    fn to_vector(&self) -> Point {
        let v = self.to - self.control;
        if v.x.abs() < 1e-10 && v.y.abs() < 1e-10 {
            self.to - self.from
        } else {
            v
        }
    }

    fn is_degenerate(&self) -> bool {
        let d = self.to - self.from;
        d.x.abs() < 1e-6 && d.y.abs() < 1e-6
    }

    fn bounding_rect(&self) -> (f32, f32, f32, f32) {
        (
            self.from.x.min(self.to.x).min(self.control.x),
            self.from.y.min(self.to.y).min(self.control.y),
            self.from.x.max(self.to.x).max(self.control.x),
            self.from.y.max(self.to.y).max(self.control.y),
        )
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Cubic {
    pub(crate) from: Point,
    pub(crate) to: Point,
    pub(crate) control1: Point,
    pub(crate) control2: Point,
}

impl SegmentTrait for Cubic {
    fn endpoints(&self) -> (Point, Point) {
        (self.from, self.to)
    }

    fn set_from(&mut self, point: Point) {
        self.from = point;
    }

    fn set_to(&mut self, point: Point) {
        self.to = point;
    }

    fn chop(&self, t: f32) -> (Cubic, Cubic) {
        let mut result = [Point::default(); 7];
        let center = NormalizedF32Exclusive::new_bounded(t);
        let arg = [self.from, self.control1, self.control2, self.to];
        path_geometry::chop_cubic_at2(&arg, center, &mut result);
        (
            Cubic {
                from: result[0],
                control1: result[1],
                control2: result[2],
                to: result[3],
            },
            Cubic {
                from: result[3],
                control1: result[4],
                control2: result[5],
                to: result[6],
            },
        )
    }

    fn reverse(&self) -> Self {
        Cubic {
            from: self.to,
            to: self.from,
            control1: self.control2,
            control2: self.control1,
        }
    }

    fn evaluate(&self, t: f32) -> Point {
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let t2 = t * t;
        let x = mt2 * mt * self.from.x
            + 3.0 * mt2 * t * self.control1.x
            + 3.0 * mt * t2 * self.control2.x
            + t2 * t * self.to.x;
        let y = mt2 * mt * self.from.y
            + 3.0 * mt2 * t * self.control1.y
            + 3.0 * mt * t2 * self.control2.y
            + t2 * t * self.to.y;
        Point::from_xy(x, y)
    }

    fn flatten(&self, tolerance: f32) -> Vec<Point> {
        let mut points = vec![self.from];
        flatten_cubic(
            self.from,
            self.control1,
            self.control2,
            self.to,
            tolerance,
            &mut points,
        );
        points
    }

    fn from_vector(&self) -> Point {
        let v = self.control1 - self.from;
        if v.x.abs() < 1e-10 && v.y.abs() < 1e-10 {
            let v2 = self.control2 - self.from;
            if v2.x.abs() < 1e-10 && v2.y.abs() < 1e-10 {
                self.to - self.from
            } else {
                v2
            }
        } else {
            v
        }
    }

    fn to_vector(&self) -> Point {
        let v = self.to - self.control2;
        if v.x.abs() < 1e-10 && v.y.abs() < 1e-10 {
            let v2 = self.to - self.control1;
            if v2.x.abs() < 1e-10 && v2.y.abs() < 1e-10 {
                self.to - self.from
            } else {
                v2
            }
        } else {
            v
        }
    }

    fn is_degenerate(&self) -> bool {
        let d = self.to - self.from;
        d.x.abs() < 1e-6 && d.y.abs() < 1e-6
    }

    fn bounding_rect(&self) -> (f32, f32, f32, f32) {
        (
            self.from
                .x
                .min(self.to.x)
                .min(self.control1.x)
                .min(self.control2.x),
            self.from
                .y
                .min(self.to.y)
                .min(self.control1.y)
                .min(self.control2.y),
            self.from
                .x
                .max(self.to.x)
                .max(self.control1.x)
                .max(self.control2.x),
            self.from
                .y
                .max(self.to.y)
                .max(self.control1.y)
                .max(self.control2.y),
        )
    }
}

/// 2次ベジェ曲線をフラット化
fn flatten_quadratic(p0: Point, p1: Point, p2: Point, tolerance: f32, points: &mut Vec<Point>) {
    // De Casteljau の誤差推定
    let dx = p0.x - 2.0 * p1.x + p2.x;
    let dy = p0.y - 2.0 * p1.y + p2.y;
    let err = dx * dx + dy * dy;
    if err <= tolerance * tolerance {
        points.push(p2);
        return;
    }
    // 中点分割
    let p01 = mid(p0, p1);
    let p12 = mid(p1, p2);
    let p012 = mid(p01, p12);
    flatten_quadratic(p0, p01, p012, tolerance, points);
    flatten_quadratic(p012, p12, p2, tolerance, points);
}

/// 3次ベジェ曲線をフラット化
fn flatten_cubic(
    p0: Point,
    p1: Point,
    p2: Point,
    p3: Point,
    tolerance: f32,
    points: &mut Vec<Point>,
) {
    // 制御点からの最大偏差を計算
    let d1x = 3.0 * p1.x - 2.0 * p0.x - p3.x;
    let d1y = 3.0 * p1.y - 2.0 * p0.y - p3.y;
    let d2x = 3.0 * p2.x - p0.x - 2.0 * p3.x;
    let d2y = 3.0 * p2.y - p0.y - 2.0 * p3.y;
    let err = (d1x * d1x + d1y * d1y).max(d2x * d2x + d2y * d2y);
    if err <= tolerance * tolerance {
        points.push(p3);
        return;
    }
    let p01 = mid(p0, p1);
    let p12 = mid(p1, p2);
    let p23 = mid(p2, p3);
    let p012 = mid(p01, p12);
    let p123 = mid(p12, p23);
    let p0123 = mid(p012, p123);
    flatten_cubic(p0, p01, p012, p0123, tolerance, points);
    flatten_cubic(p0123, p123, p23, p3, tolerance, points);
}

fn mid(a: Point, b: Point) -> Point {
    Point::from_xy((a.x + b.x) * 0.5, (a.y + b.y) * 0.5)
}

/// パスセグメントの列挙型
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PathSegment {
    Line(Line),
    Quadratic(Quadratic),
    Cubic(Cubic),
}

impl PathSegment {
    pub(crate) fn endpoints(&self) -> (Point, Point) {
        match self {
            PathSegment::Line(l) => l.endpoints(),
            PathSegment::Quadratic(q) => q.endpoints(),
            PathSegment::Cubic(c) => c.endpoints(),
        }
    }

    pub(crate) fn set_from(&mut self, point: Point) {
        match self {
            PathSegment::Line(l) => l.set_from(point),
            PathSegment::Quadratic(q) => q.set_from(point),
            PathSegment::Cubic(c) => c.set_from(point),
        }
    }

    pub(crate) fn set_to(&mut self, point: Point) {
        match self {
            PathSegment::Line(l) => l.set_to(point),
            PathSegment::Quadratic(q) => q.set_to(point),
            PathSegment::Cubic(c) => c.set_to(point),
        }
    }

    pub(crate) fn chop(&self, t: f32) -> (PathSegment, PathSegment) {
        match self {
            PathSegment::Line(l) => {
                let (a, b) = l.chop(t);
                (PathSegment::Line(a), PathSegment::Line(b))
            }
            PathSegment::Quadratic(q) => {
                let (a, b) = q.chop(t);
                (PathSegment::Quadratic(a), PathSegment::Quadratic(b))
            }
            PathSegment::Cubic(c) => {
                let (a, b) = c.chop(t);
                (PathSegment::Cubic(a), PathSegment::Cubic(b))
            }
        }
    }

    pub(crate) fn reverse(&self) -> Self {
        match self {
            PathSegment::Line(l) => PathSegment::Line(l.reverse()),
            PathSegment::Quadratic(q) => PathSegment::Quadratic(q.reverse()),
            PathSegment::Cubic(c) => PathSegment::Cubic(c.reverse()),
        }
    }

    pub(crate) fn evaluate(&self, t: f32) -> Point {
        match self {
            PathSegment::Line(l) => l.evaluate(t),
            PathSegment::Quadratic(q) => q.evaluate(t),
            PathSegment::Cubic(c) => c.evaluate(t),
        }
    }

    pub(crate) fn flatten(&self, tolerance: f32) -> Vec<Point> {
        match self {
            PathSegment::Line(l) => l.flatten(tolerance),
            PathSegment::Quadratic(q) => q.flatten(tolerance),
            PathSegment::Cubic(c) => c.flatten(tolerance),
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn from_vector(&self) -> Point {
        match self {
            PathSegment::Line(l) => l.from_vector(),
            PathSegment::Quadratic(q) => q.from_vector(),
            PathSegment::Cubic(c) => c.from_vector(),
        }
    }

    pub(crate) fn to_vector(&self) -> Point {
        match self {
            PathSegment::Line(l) => l.to_vector(),
            PathSegment::Quadratic(q) => q.to_vector(),
            PathSegment::Cubic(c) => c.to_vector(),
        }
    }

    pub(crate) fn is_degenerate(&self) -> bool {
        match self {
            PathSegment::Line(l) => l.is_degenerate(),
            PathSegment::Quadratic(q) => q.is_degenerate(),
            PathSegment::Cubic(c) => c.is_degenerate(),
        }
    }

    pub(crate) fn is_same_or_reversed(&self, other: &Self) -> bool {
        self == other || self == &other.reverse()
    }

    #[allow(dead_code)]
    pub(crate) fn bounding_rect(&self) -> (f32, f32, f32, f32) {
        match self {
            PathSegment::Line(l) => l.bounding_rect(),
            PathSegment::Quadratic(q) => q.bounding_rect(),
            PathSegment::Cubic(c) => c.bounding_rect(),
        }
    }
}
