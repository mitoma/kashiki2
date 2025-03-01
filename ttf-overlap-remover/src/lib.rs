use tiny_skia_path::{path_geometry, NormalizedF32Exclusive, Path, Point, Rect};

/// Point を PathSegment に変換する
fn point_to_dot(point: Point) -> PathSegment {
    let (x, y) = (point.x, point.y);
    PathSegment::Line(Line {
        from: point,
        to: Point {
            x: x + f32::EPSILON,
            y: y + f32::EPSILON,
        },
    })
}

/// Path を PathSegment に変換する
fn path_to_path_segments(path: Path) -> Vec<PathSegment> {
    let mut results = Vec::new();

    let mut start_point = Point::default();
    for segment in path.segments() {
        match segment {
            tiny_skia_path::PathSegment::MoveTo(point) => start_point = point,
            tiny_skia_path::PathSegment::LineTo(point) => {
                results.push(PathSegment::Line(Line {
                    from: start_point,
                    to: point,
                }));
                start_point = point;
            }
            tiny_skia_path::PathSegment::QuadTo(point, point1) => {
                results.push(PathSegment::Quadratic(Quadratic {
                    from: start_point,
                    to: point,
                    control: point1,
                }));
                start_point = point;
            }
            tiny_skia_path::PathSegment::CubicTo(point, point1, point2) => {
                results.push(PathSegment::Cubic(Cubic {
                    from: start_point,
                    to: point,
                    control1: point1,
                    control2: point2,
                }));
                start_point = point;
            }
            tiny_skia_path::PathSegment::Close => {}
        }
    }
    results
}

// PathSegment に備わっていてほしい共通操作を集めた trait
trait SegmentTrait
where
    Self: Sized + Clone,
{
    fn move_to(&self, point: Point) -> Self;
    fn set_from(&mut self, point: Point);
    fn set_to(&mut self, point: Point);
    fn endpoints(&self) -> (Point, Point);
    fn rect(&self) -> Rect;
    fn chop_harf(&self) -> (Self, Self);
    fn chop(&self, position: f32) -> (Self, Self);
    fn to_path_segment(self) -> PathSegment;
}

#[derive(Debug, Clone)]
struct Line {
    from: Point,
    to: Point,
}

impl SegmentTrait for Line {
    fn move_to(&self, point: Point) -> Self {
        Line {
            from: self.from + point,
            to: self.to + point,
        }
    }

    fn set_from(&mut self, point: Point) {
        self.from = point;
    }

    fn set_to(&mut self, point: Point) {
        self.to = point;
    }

    fn endpoints(&self) -> (Point, Point) {
        (self.from, self.to)
    }

    fn rect(&self) -> Rect {
        let min_x = self.from.x.min(self.to.x);
        let min_y = self.from.y.min(self.to.y);
        let max_x = self.from.x.max(self.to.x);
        let max_y = self.from.y.max(self.to.y);
        Rect::from_xywh(min_x, min_y, max_x - min_x, max_y - min_y).unwrap()
    }

    fn chop_harf(&self) -> (Line, Line) {
        self.chop(0.5)
    }

    fn chop(&self, position: f32) -> (Line, Line) {
        let new_x = self.from.x + position * (self.to.x - self.from.x);
        let new_y = self.from.y + position * (self.to.y - self.from.y);
        let mid_point = Point::from_xy(new_x, new_y);
        (
            Line {
                from: self.from,
                to: mid_point,
            },
            Line {
                from: mid_point,
                to: self.to,
            },
        )
    }

    fn to_path_segment(self) -> PathSegment {
        PathSegment::Line(self)
    }
}

#[derive(Debug, Clone)]

struct Quadratic {
    from: Point,
    to: Point,
    control: Point,
}

impl SegmentTrait for Quadratic {
    fn move_to(&self, point: Point) -> Self {
        Quadratic {
            from: self.from + point,
            to: self.to + point,
            control: self.control + point,
        }
    }

    fn set_from(&mut self, point: Point) {
        self.from = point;
    }

    fn set_to(&mut self, point: Point) {
        self.to = point;
    }

    fn endpoints(&self) -> (Point, Point) {
        (self.from, self.to)
    }

    fn rect(&self) -> Rect {
        let min_x = self.from.x.min(self.to.x).min(self.control.x);
        let min_y = self.from.y.min(self.to.y).min(self.control.y);
        let max_x = self.from.x.max(self.to.x).max(self.control.x);
        let max_y = self.from.y.max(self.to.y).max(self.control.y);
        Rect::from_xywh(min_x, min_y, max_x - min_x, max_y - min_y).unwrap()
    }

    fn chop_harf(&self) -> (Quadratic, Quadratic) {
        self.chop(0.5)
    }

    fn chop(&self, position: f32) -> (Quadratic, Quadratic) {
        let mut result = [Point::default(); 5];
        let center = NormalizedF32Exclusive::new_bounded(position);
        let arg = [self.from, self.control, self.to];
        path_geometry::chop_quad_at(&arg, center, &mut result);
        (
            Quadratic {
                from: result[0],
                to: result[2],
                control: result[1],
            },
            Quadratic {
                from: result[2],
                to: result[4],
                control: result[3],
            },
        )
    }

    fn to_path_segment(self) -> PathSegment {
        PathSegment::Quadratic(self)
    }
}

#[derive(Debug, Clone)]
struct Cubic {
    from: Point,
    to: Point,
    control1: Point,
    control2: Point,
}

impl SegmentTrait for Cubic {
    fn move_to(&self, point: Point) -> Self {
        Cubic {
            from: self.from + point,
            to: self.to + point,
            control1: self.control1 + point,
            control2: self.control2 + point,
        }
    }

    fn set_from(&mut self, point: Point) {
        self.from = point;
    }

    fn set_to(&mut self, point: Point) {
        self.to = point;
    }

    fn endpoints(&self) -> (Point, Point) {
        (self.from, self.to)
    }

    fn rect(&self) -> Rect {
        let min_x = self
            .from
            .x
            .min(self.to.x)
            .min(self.control1.x)
            .min(self.control2.x);
        let min_y = self
            .from
            .y
            .min(self.to.y)
            .min(self.control1.y)
            .min(self.control2.y);
        let max_x = self
            .from
            .x
            .max(self.to.x)
            .max(self.control1.x)
            .max(self.control2.x);
        let max_y = self
            .from
            .y
            .max(self.to.y)
            .max(self.control1.y)
            .max(self.control2.y);
        Rect::from_xywh(min_x, min_y, max_x - min_x, max_y - min_y).unwrap()
    }

    fn chop_harf(&self) -> (Cubic, Cubic) {
        self.chop(0.5)
    }

    fn chop(&self, position: f32) -> (Cubic, Cubic) {
        let mut result = [Point::default(); 7];
        let center = NormalizedF32Exclusive::new_bounded(position);
        let arg = [self.from, self.control1, self.control2, self.to];
        path_geometry::chop_cubic_at2(&arg, center, &mut result);
        (
            Cubic {
                from: result[0],
                to: result[3],
                control1: result[1],
                control2: result[2],
            },
            Cubic {
                from: result[3],
                to: result[6],
                control1: result[4],
                control2: result[5],
            },
        )
    }

    fn to_path_segment(self) -> PathSegment {
        PathSegment::Cubic(self)
    }
}

#[derive(Debug, Clone)]
enum PathSegment {
    Line(Line),
    Quadratic(Quadratic),
    Cubic(Cubic),
}

impl PathSegment {
    fn move_to(&self, point: Point) -> Self {
        match self {
            PathSegment::Line(line) => PathSegment::Line(line.move_to(point)),
            PathSegment::Quadratic(quad) => PathSegment::Quadratic(quad.move_to(point)),
            PathSegment::Cubic(cubic) => PathSegment::Cubic(cubic.move_to(point)),
        }
    }

    fn set_from(&mut self, point: Point) {
        match self {
            PathSegment::Line(line) => line.set_from(point),
            PathSegment::Quadratic(quad) => quad.set_from(point),
            PathSegment::Cubic(cubic) => cubic.set_from(point),
        }
    }

    fn set_to(&mut self, point: Point) {
        match self {
            PathSegment::Line(line) => line.set_to(point),
            PathSegment::Quadratic(quad) => quad.set_to(point),
            PathSegment::Cubic(cubic) => cubic.set_to(point),
        }
    }

    fn endpoints(&self) -> (Point, Point) {
        match self {
            PathSegment::Line(line) => line.endpoints(),
            PathSegment::Quadratic(quad) => quad.endpoints(),
            PathSegment::Cubic(cubic) => cubic.endpoints(),
        }
    }

    fn rect(&self) -> Rect {
        match self {
            PathSegment::Line(line) => line.rect(),
            PathSegment::Quadratic(quad) => quad.rect(),
            PathSegment::Cubic(cubic) => cubic.rect(),
        }
    }

    fn chop_harf(&self) -> (PathSegment, PathSegment) {
        self.chop(0.5)
    }

    /// position で指定された位置でセグメントを分割する
    /// position は 0.0 から 1.0 の範囲で指定する
    fn chop(&self, position: f32) -> (PathSegment, PathSegment) {
        match self {
            PathSegment::Line(line) => {
                let (line1, line2) = line.chop(position);
                (PathSegment::Line(line1), PathSegment::Line(line2))
            }
            PathSegment::Quadratic(quad) => {
                let (quad1, quad2) = quad.chop(position);
                (PathSegment::Quadratic(quad1), PathSegment::Quadratic(quad2))
            }
            PathSegment::Cubic(cubic) => {
                let (cubic1, cubic2) = cubic.chop(position);
                (PathSegment::Cubic(cubic1), PathSegment::Cubic(cubic2))
            }
        }
    }
}

fn split_all_paths(paths: Vec<PathSegment>) -> Vec<PathSegment> {
    let mut paths = paths.clone();
    let mut result = Vec::new();
    let mut debug_counter = 0;

    let mut has_cross = true;
    while has_cross {
        'outer: {
            for i in 0..paths.len() {
                for j in i + 1..paths.len() {
                    if let Some((a, b)) = split_line_on_cross_point(&paths[i], &paths[j]) {
                        has_cross = true;
                        result.extend(a);
                        result.extend(b);
                        if i + 1 != j {
                            result.extend_from_slice(paths[i + 1..j].as_ref());
                        }
                        result.extend_from_slice(paths[j + 1..].as_ref());
                        paths = result;
                        result = Vec::new();
                        break 'outer;
                    }
                }
                result.push(paths[i].clone());
            }
            has_cross = false;
        }
    }
    result
}

// 二つのセグメントが交差しているかを判定し、交差している場合はその交差点で二つのセグメントとをそれぞれ分割する
fn split_line_on_cross_point(
    a: &PathSegment,
    b: &PathSegment,
) -> Option<(Vec<PathSegment>, Vec<PathSegment>)> {
    let cross_points = cross_point(a, b)
        .into_iter()
        // 端点同士が交点となる場合は分割対象外
        .filter(|cp| {
            !([0.0, 1.0].contains(&cp.a_position.abs())
                && [0.0, 1.0].contains(&cp.b_position.abs()))
        })
        .collect::<Vec<_>>();
    if cross_points.is_empty() {
        return None;
    }

    let mut a_sorted = cross_points.clone();
    a_sorted.sort_by(|l, r| l.a_position.partial_cmp(&r.a_position).unwrap());
    let (mut a_result, last, _) = a_sorted.iter().fold(
        (vec![], a.clone(), 0.0f32),
        |(mut result, target_path, consumed), cp| {
            let length = 1.0 - consumed;
            let next_gain = cp.a_position - consumed;
            let chop_point = next_gain / length;
            let (mut pre, mut post) = target_path.chop(chop_point);
            // 単に chop しただけだと誤差の都合で導出した交点と一致しない場合があるので、導出した交点に置き換える
            pre.set_to(cp.point);
            post.set_from(cp.point);
            result.push(pre);
            (result, post, consumed + cp.a_position)
        },
    );
    a_result.push(last);

    let mut b_sorted = cross_points.clone();
    b_sorted.sort_by(|l, r| l.b_position.partial_cmp(&r.b_position).unwrap());
    let (mut b_result, last, _) = b_sorted.iter().fold(
        (vec![], b.clone(), 0.0f32),
        |(mut result, target_path, consumed), cp| {
            let length = 1.0 - consumed;
            let next_gain = cp.b_position - consumed;
            let chop_point = next_gain / length;
            let (mut pre, mut post) = target_path.chop(chop_point);
            pre.set_to(cp.point);
            post.set_from(cp.point);
            result.push(pre);
            (result, post, consumed + cp.b_position)
        },
    );
    b_result.push(last);

    Some((a_result, b_result))
}

const EPSILON: f32 = 0.001;
fn cross_point(a: &PathSegment, b: &PathSegment) -> Vec<CrossPoint> {
    // 二つのセグメントが交差しているかどうかを矩形で判定
    if a.rect().intersect(&b.rect()).is_none() {
        return vec![];
    };
    let result = match (a, b) {
        (PathSegment::Line(a), PathSegment::Line(b)) => {
            if let Some(point) = cross_point_line(a, b) {
                vec![point]
            } else {
                vec![]
            }
        }
        (PathSegment::Line(line), PathSegment::Quadratic(quad)) => closs_point_inner(line, quad),
        (PathSegment::Quadratic(quad), PathSegment::Line(line)) => closs_point_inner(quad, line),
        (PathSegment::Line(line), PathSegment::Cubic(cubic)) => closs_point_inner(line, cubic),
        (PathSegment::Cubic(cubic), PathSegment::Line(line)) => closs_point_inner(cubic, line),
        (PathSegment::Quadratic(quadratic), PathSegment::Cubic(cubic)) => {
            closs_point_inner(quadratic, cubic)
        }
        (PathSegment::Cubic(cubic), PathSegment::Quadratic(quadratic)) => {
            closs_point_inner(cubic, quadratic)
        }
        (PathSegment::Quadratic(quadratic1), PathSegment::Quadratic(quadratic2)) => {
            closs_point_inner(quadratic1, quadratic2)
        }
        (PathSegment::Cubic(cubic1), PathSegment::Cubic(cubic2)) => {
            closs_point_inner(cubic1, cubic2)
        }
    };
    result
}

#[derive(Debug, Clone)]
struct CrossPoint {
    point: Point,
    // 交点が線分のどの位置にあるかを示す。0.0 から 1.0 の範囲で示す
    a_position: f32,
    b_position: f32,
}

#[inline]
fn cross_point_line(a: &Line, b: &Line) -> Option<CrossPoint> {
    // 直線同士の交点を求める
    let denom =
        (b.to.y - b.from.y) * (a.to.x - a.from.x) - (b.to.x - b.from.x) * (a.to.y - a.from.y);
    if denom == 0.0 {
        return None; // 平行な場合は交点なし
    }
    let ua = ((b.to.x - b.from.x) * (a.from.y - b.from.y)
        - (b.to.y - b.from.y) * (a.from.x - b.from.x))
        / denom;
    let ub = ((a.to.x - a.from.x) * (a.from.y - b.from.y)
        - (a.to.y - a.from.y) * (a.from.x - b.from.x))
        / denom;
    if 0.0 <= ua && ua < 1.0 && 0.0 <= ub && ub < 1.0 {
        let x = a.from.x + ua * (a.to.x - a.from.x);
        let y = a.from.y + ua * (a.to.y - a.from.y);
        Some(CrossPoint {
            point: Point::from_xy(x, y),
            a_position: ua,
            b_position: ub,
        })
    } else {
        None // 線分上に交点がない場合
    }
}

#[inline]
fn closs_point_inner<T: SegmentTrait, U: SegmentTrait>(a: &T, b: &U) -> Vec<CrossPoint> {
    struct StackItem<T, U> {
        a: T,
        a_position: f32,
        b: U,
        b_position: f32,
        depth: u32,
    }

    let mut stack: Vec<StackItem<T, U>> = vec![StackItem {
        a: a.clone(),
        a_position: 0.0,
        b: b.clone(),
        b_position: 0.0,
        depth: 0,
    }];
    let mut points = Vec::new();

    while let Some(StackItem {
        a,
        a_position,
        b,
        b_position,
        depth,
    }) = stack.pop()
    {
        let intersect = a.rect().intersect(&b.rect());
        if let Some(intersect) = intersect {
            if intersect.width() < EPSILON && intersect.height() < EPSILON {
                let gain = 1.0 / (2u32.pow(depth) as f32);
                let (a_from, a_to) = a.endpoints();
                let (b_from, b_to) = b.endpoints();
                if let Some(point) = cross_point_line(
                    &Line {
                        from: a_from,
                        to: a_to,
                    },
                    &Line {
                        from: b_from,
                        to: b_to,
                    },
                ) {
                    points.push(CrossPoint {
                        point: point.point,
                        a_position: a_position + point.a_position * gain,
                        b_position: b_position + point.b_position * gain,
                    });
                }
            } else {
                let depth = depth + 1;
                let gain = 1.0 / (2u32.pow(depth) as f32);
                let (a1, a2) = a.chop_harf();
                let (b1, b2) = b.chop_harf();
                stack.push(StackItem {
                    a: a1.clone(),
                    a_position,
                    b: b1.clone(),
                    b_position,
                    depth,
                });
                stack.push(StackItem {
                    a: a1.clone(),
                    a_position,
                    b: b2.clone(),
                    b_position: b_position + gain,
                    depth,
                });
                stack.push(StackItem {
                    a: a2.clone(),
                    a_position: a_position + gain,
                    b: b1.clone(),
                    b_position,
                    depth,
                });
                stack.push(StackItem {
                    a: a2.clone(),
                    a_position: a_position + gain,
                    b: b2.clone(),
                    b_position: b_position + gain,
                    depth,
                });
            }
        }
    }
    points
}

#[cfg(test)]
mod tests {
    use std::{f32, vec};

    use rustybuzz::{ttf_parser::OutlineBuilder, Face};
    use tiny_skia::{Paint, Path, Pixmap};
    use tiny_skia_path::{
        path_geometry, NormalizedF32Exclusive, PathBuilder, Point, Stroke, Transform,
    };

    use crate::{
        cross_point, cross_point_line, path_to_path_segments, split_all_paths,
        split_line_on_cross_point, Cubic, Line, PathSegment, Quadratic, EPSILON,
    };

    #[test]
    fn test_cross_point_lines_intersect() {
        let p1 = Point::from_xy(0.0, 0.0);
        let p2 = Point::from_xy(2.0, 2.0);
        let p3 = Point::from_xy(0.0, 2.0);
        let p4 = Point::from_xy(2.0, 0.0);

        let segment1 = PathSegment::Line(Line { from: p1, to: p2 });
        let segment2 = PathSegment::Line(Line { from: p3, to: p4 });

        let result = cross_point(&segment1, &segment2);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].point, Point::from_xy(1.0, 1.0));
    }

    #[test]
    fn test_cross_point_lines_parallel() {
        let p1 = Point::from_xy(0.0, 0.0);
        let p2 = Point::from_xy(2.0, 2.0);
        let p3 = Point::from_xy(0.0, 1.0);
        let p4 = Point::from_xy(2.0, 3.0);

        let segment1 = PathSegment::Line(Line { from: p1, to: p2 });
        let segment2 = PathSegment::Line(Line { from: p3, to: p4 });

        let result = cross_point(&segment1, &segment2);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_cross_point_lines_no_intersect() {
        let p1 = Point::from_xy(0.0, 0.0);
        let p2 = Point::from_xy(1.0, 1.0);
        let p3 = Point::from_xy(2.0, 2.0);
        let p4 = Point::from_xy(3.0, 3.0);

        let segment1 = PathSegment::Line(Line { from: p1, to: p2 });
        let segment2 = PathSegment::Line(Line { from: p3, to: p4 });

        let result = cross_point(&segment1, &segment2);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_cross_point_line_quadratic() {
        let p1 = Point::from_xy(0.0, 0.0);
        let p2 = Point::from_xy(2.0, 2.0);
        let q1 = Point::from_xy(0.0, 2.0);
        let q2 = Point::from_xy(2.0, 0.0);
        let control = Point::from_xy(1.0, 3.0);

        let segment1 = PathSegment::Line(Line { from: p1, to: p2 });
        let segment2 = PathSegment::Quadratic(Quadratic {
            from: q1,
            to: q2,
            control,
        });

        let result = cross_point(&segment1, &segment2);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_cross_point_line_quadratic_one_intersection() {
        let p1 = Point::from_xy(0.0, 0.0);
        let p2 = Point::from_xy(2.0, 2.0);
        let q1 = Point::from_xy(0.0, 2.0);
        let q2 = Point::from_xy(2.0, 0.0);
        let control = Point::from_xy(1.0, 1.0);

        let segment1 = PathSegment::Line(Line { from: p1, to: p2 });
        let segment2 = PathSegment::Quadratic(Quadratic {
            from: q1,
            to: q2,
            control,
        });

        path_segments_to_image(vec![&segment1, &segment2], vec![]);

        let result = cross_point(&segment1, &segment2);
        println!("{:?}", result);
        assert_eq!(result.len(), 1);
        let intersection = Point::from_xy(1.0, 1.0);
        assert!((result[0].point.x - intersection.x).abs() < 0.01);
        assert!((result[0].point.y - intersection.y).abs() < 0.01);
    }

    #[test]
    fn test_cross_point_line_quadratic_two_intersection() {
        let p1 = Point::from_xy(0.0, 1.0);
        let p2 = Point::from_xy(2.0, 1.5);
        let q1 = Point::from_xy(0.0, 2.0);
        let q2 = Point::from_xy(2.0, 2.0);
        let control = Point::from_xy(1.0, 0.0);

        let segment1 = PathSegment::Line(Line { from: p1, to: p2 });
        let segment2 = PathSegment::Quadratic(Quadratic {
            from: q1,
            to: q2,
            control,
        });

        path_segments_to_image(vec![&segment1, &segment2], vec![]);

        let result = cross_point(&segment1, &segment2);
        println!("{:?}", result);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_cross_point_line_quadratic_two_intersections() {
        let p1 = Point::from_xy(0.0, 1.0);
        let p2 = Point::from_xy(2.0, 1.0);
        let q1 = Point::from_xy(0.0, 0.0);
        let q2 = Point::from_xy(2.0, 2.0);
        let control = Point::from_xy(1.0, 3.0);

        let segment1 = PathSegment::Line(Line { from: p1, to: p2 });
        let segment2 = PathSegment::Quadratic(Quadratic {
            from: q1,
            to: q2,
            control,
        });

        let result = cross_point(&segment1, &segment2);
        path_segments_to_image(
            vec![&segment1, &segment2],
            result.iter().map(|cp| &cp.point).collect(),
        );
        assert_eq!(result.len(), 1);
        let intersection = Point::from_xy(0.5, 1.0);
        assert!((result[0].point - intersection).x < 0.01);
        assert!((result[0].point - intersection).y < 0.01);
    }

    #[test]
    fn test_cross_point_line_cubic() {
        let l1 = Point::from_xy(0.0, 0.8);
        let l2 = Point::from_xy(2.0, 1.2);
        let line_seg = PathSegment::Line(Line { from: l1, to: l2 });

        let p1 = Point::from_xy(0.0, 1.0);
        let p2 = Point::from_xy(2.0, 1.0);
        let c1 = Point::from_xy(0.5, 0.0);
        let c2 = Point::from_xy(1.7, 2.0);
        let cubic_seg = PathSegment::Cubic(Cubic {
            from: p1,
            to: p2,
            control1: c1,
            control2: c2,
        });

        let result = cross_point(&line_seg, &cubic_seg);
        path_segments_to_image(
            vec![&line_seg, &cubic_seg],
            result.iter().map(|cp| &cp.point).collect(),
        );
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_cross_point_cubic_cubic() {
        let p1 = Point::from_xy(0.0, 2.0);
        let p2 = Point::from_xy(2.0, 2.0);
        let c1 = Point::from_xy(0.5, 2.0);
        let c2 = Point::from_xy(1.7, 0.0);
        let cubic_seg1 = PathSegment::Cubic(Cubic {
            from: p1,
            to: p2,
            control1: c1,
            control2: c2,
        });

        let p1 = Point::from_xy(0.0, 1.0);
        let p2 = Point::from_xy(2.0, 1.0);
        let c1 = Point::from_xy(0.5, 0.0);
        let c2 = Point::from_xy(1.7, 2.0);
        let cubic_seg2 = PathSegment::Cubic(Cubic {
            from: p1,
            to: p2,
            control1: c1,
            control2: c2,
        });

        let result = cross_point(&cubic_seg1, &cubic_seg2);
        path_segments_to_image(
            vec![&cubic_seg1, &cubic_seg2],
            result.iter().map(|cp| &cp.point).collect(),
        );
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_chop() {
        let q1 = Point::from_xy(0.0, 0.0);
        let q2 = Point::from_xy(2.0, 2.0);
        let control = Point::from_xy(1.0, 3.0);

        let segment = PathSegment::Quadratic(Quadratic {
            from: q1,
            to: q2,
            control,
        });

        let arg = [q1, control, q2];
        let mut result: [Point; 5] = Default::default();
        let center = NormalizedF32Exclusive::new_bounded(0.5);

        let _ = path_geometry::chop_quad_at(&arg, center, &mut result);
        let pre = PathSegment::Quadratic(Quadratic {
            from: result[0],
            to: result[2],
            control: result[1],
        });
        let post = PathSegment::Quadratic(Quadratic {
            from: result[2],
            to: result[4],
            control: result[3],
        });

        path_segments_to_image(vec![&segment, &pre, &post], vec![]);
    }

    #[test]
    fn test_chop2() {
        let p1 = Point::from_xy(1.0, 0.0);
        let p2 = Point::from_xy(0.0, 2.0);
        let line_seg = PathSegment::Line(Line { from: p1, to: p2 });
        let (line1, line2) = line_seg.chop(0.3);

        let q1 = Point::from_xy(0.0, 0.0);
        let q2 = Point::from_xy(2.0, 2.0);
        let control = Point::from_xy(1.0, 3.0);

        let quad_seg = PathSegment::Quadratic(Quadratic {
            from: q1,
            to: q2,
            control,
        });

        let (quad1, quad2) = quad_seg.chop(0.5);

        path_segments_to_image(vec![&line1, &line2, &quad1, &quad2], vec![]);
    }

    #[test]
    fn test_cubic() {
        let p1 = Point::from_xy(0.0, 1.0);
        let p2 = Point::from_xy(2.0, 1.0);
        let c1 = Point::from_xy(0.5, 0.0);
        let c2 = Point::from_xy(1.7, 2.0);
        let line_seg = PathSegment::Cubic(Cubic {
            from: p1,
            to: p2,
            control1: c1,
            control2: c2,
        });
        let (line1, line2) = line_seg.chop(0.3);
        path_segments_to_image(vec![&line1, &line2], vec![]);
    }

    #[test]
    fn test_font() {
        let font_file = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");
        let face: Face = Face::from_slice(font_file, 0).unwrap();
        let glyph_id = face.glyph_index('🐢').unwrap();
        let mut path_builder = MyPathBuilder::new();
        face.outline_glyph(glyph_id, &mut path_builder).unwrap();
        let paths = path_builder.paths();

        let segments: Vec<PathSegment> = paths
            .iter()
            .map(|path| path_to_path_segments(path.clone()))
            .flatten()
            .collect();
        segments
            .iter()
            .map(|segment| println!("{:?}", segment.endpoints()))
            .for_each(drop);
        println!("{:?}", segments.len());

        let mut dots = vec![];
        for i in 0..segments.len() {
            for j in i + 1..segments.len() {
                let result = cross_point(&segments[i], &segments[j]);
                if !result.is_empty() {
                    dots.extend(result);
                }
            }
        }
        path_segments_to_image(
            segments.iter().collect(),
            dots.iter().map(|cp| &cp.point).collect(),
        );
    }

    #[test]
    fn test_font2() {
        let font_file = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");
        let face: Face = Face::from_slice(font_file, 0).unwrap();
        let glyph_id = face.glyph_index('🐢').unwrap();
        let mut path_builder = MyPathBuilder::new();
        face.outline_glyph(glyph_id, &mut path_builder).unwrap();
        let paths = path_builder.paths();

        let segments: Vec<PathSegment> = paths
            .iter()
            .map(|path| path_to_path_segments(path.clone()))
            .flatten()
            .collect();

        let splitted_paths = split_all_paths(segments);
        let splitted_paths: Vec<_> = splitted_paths
            .iter()
            .enumerate()
            .map(|(i, segment)| segment.move_to(Point::from_xy(0.0, i as f32 * 1.0)))
            .collect();

        path_segments_to_image(splitted_paths.iter().collect(), vec![]);
    }

    // split のテスト

    #[test]
    fn test_split_quad_quad() {
        let quad_seg1 = PathSegment::Quadratic(Quadratic {
            from: Point::from_xy(0.0, 0.0),
            to: Point::from_xy(2.0, 2.0),
            control: Point::from_xy(1.5, 2.0),
        });

        let quad_seg2 = PathSegment::Quadratic(Quadratic {
            from: Point::from_xy(0.0, 2.0),
            to: Point::from_xy(2.0, 0.0),
            control: Point::from_xy(0.5, 0.0),
        });

        let (split1, split2) = split_line_on_cross_point(&quad_seg1, &quad_seg2).unwrap();
        let mut result_seg = vec![];
        result_seg.extend(split1.iter());
        result_seg.extend(split2.iter());
        let moved_result: Vec<PathSegment> = result_seg
            .iter()
            .map(|seg| seg.move_to(Point::from_xy(0.0, 0.1)))
            .collect();

        let mut draw_vec = vec![&quad_seg1, &quad_seg2];
        draw_vec.extend(moved_result.iter());

        path_segments_to_image(draw_vec, vec![]);
        println!("{:?}", split1);
        println!("{:?}", split2);
        assert_eq!(split1.len(), 2);
        assert_eq!(split2.len(), 2);
    }

    #[test]
    fn test_split_cubic_cubic() {
        let p1 = Point::from_xy(0.0, 2.0);
        let p2 = Point::from_xy(2.0, 2.0);
        let c1 = Point::from_xy(0.5, 2.0);
        let c2 = Point::from_xy(1.7, 0.0);
        let cubic_seg1 = PathSegment::Cubic(Cubic {
            from: p1,
            to: p2,
            control1: c1,
            control2: c2,
        });

        let p1 = Point::from_xy(0.0, 1.0);
        let p2 = Point::from_xy(2.0, 1.0);
        let c1 = Point::from_xy(0.5, 0.0);
        let c2 = Point::from_xy(1.7, 2.0);
        let cubic_seg2 = PathSegment::Cubic(Cubic {
            from: p1,
            to: p2,
            control1: c1,
            control2: c2,
        });

        let (split1, split2) = split_line_on_cross_point(&cubic_seg1, &cubic_seg2).unwrap();
        let mut result_seg = vec![];
        result_seg.extend(split1.iter());
        result_seg.extend(split2.iter());

        assert_eq!(result_seg.len(), 6);

        let moved_result: Vec<PathSegment> = result_seg
            .iter()
            .enumerate()
            .map(|(i, seg)| {
                seg.move_to(Point::from_xy(
                    0.0,
                    2.0 + if i % 2 == 0 { 0.1 } else { 0.0 },
                ))
            })
            .collect();

        let mut draw_vec = vec![&cubic_seg1, &cubic_seg2];
        draw_vec.extend(moved_result.iter());

        path_segments_to_image(draw_vec, vec![]);
    }

    #[test]
    fn test_split_cubic_cubic2() {
        let p1 = Point::from_xy(0.0, 2.0);
        let p2 = Point::from_xy(2.0, 2.0);
        let c1 = Point::from_xy(0.5, 2.0);
        let c2 = Point::from_xy(1.7, 0.0);
        let cubic_seg1 = PathSegment::Cubic(Cubic {
            from: p1,
            to: p2,
            control1: c1,
            control2: c2,
        });

        let p1 = Point::from_xy(0.0, 1.0);
        let p2 = Point::from_xy(2.0, 1.0);
        let c1 = Point::from_xy(0.5, 0.0);
        let c2 = Point::from_xy(1.7, 2.0);
        let cubic_seg2 = PathSegment::Cubic(Cubic {
            from: p1,
            to: p2,
            control1: c1,
            control2: c2,
        });

        let (split1, split2) = split_line_on_cross_point(&cubic_seg1, &cubic_seg2).unwrap();
        let mut result_seg = vec![];
        result_seg.extend(split1.iter());
        result_seg.extend(split2.iter());
        assert_eq!(result_seg.len(), 6);

        for i in 0..result_seg.len() {
            for j in i + 1..result_seg.len() {
                let result = cross_point(&result_seg[i], &result_seg[j]);

                result.iter().for_each(|cp| {
                    println!("{:?}", cp);
                });
            }
        }
    }

    #[test]
    fn test_split_all_paths() {
        env_logger::builder().is_test(true).try_init().unwrap();
        let p1 = Point::from_xy(0.0, 2.0);
        let p2 = Point::from_xy(2.0, 2.0);
        let c1 = Point::from_xy(0.5, 2.0);
        let c2 = Point::from_xy(1.7, 0.0);
        let cubic_seg1 = PathSegment::Cubic(Cubic {
            from: p1,
            to: p2,
            control1: c1,
            control2: c2,
        });

        let p1 = Point::from_xy(0.0, 1.0);
        let p2 = Point::from_xy(2.0, 1.0);
        let c1 = Point::from_xy(0.5, 0.0);
        let c2 = Point::from_xy(1.7, 2.0);
        let cubic_seg2 = PathSegment::Cubic(Cubic {
            from: p1,
            to: p2,
            control1: c1,
            control2: c2,
        });

        let line_seg1 = PathSegment::Line(Line {
            from: Point::from_xy(0.0, 0.0),
            to: Point::from_xy(2.0, 2.0),
        });

        let mut draw_seg = vec![];
        let segments = vec![cubic_seg1, cubic_seg2, line_seg1];

        draw_seg.extend(segments.iter());
        let result = split_all_paths(segments.clone());

        let moved_result: Vec<PathSegment> = result
            .iter()
            .enumerate()
            .map(|(i, seg)| seg.move_to(Point::from_xy(0.0, 3.0)))
            .collect();

        draw_seg.extend(moved_result.iter());

        path_segments_to_image(draw_seg, vec![]);
        assert_eq!(result.len(), 11);
    }

    #[test]
    fn test_split_all_paths2() {
        let line_seg1 = PathSegment::Line(Line {
            from: Point::from_xy(1.0, 0.0),
            to: Point::from_xy(1.0, 4.0),
        });
        let line_seg2 = PathSegment::Line(Line {
            from: Point::from_xy(3.0, 0.0),
            to: Point::from_xy(3.0, 4.0),
        });
        let line_seg3 = PathSegment::Cubic(Cubic {
            from: Point::from_xy(0.0, 1.0),
            to: Point::from_xy(4.0, 1.0),
            control1: Point::from_xy(1.0, 0.0),
            control2: Point::from_xy(3.0, 10.0),
        });
        let line_seg4 = PathSegment::Line(Line {
            from: Point::from_xy(0.0, 3.0),
            to: Point::from_xy(4.0, 3.0),
        });

        let mut draw_seg = vec![];
        let segments = vec![line_seg3, line_seg1, line_seg2, line_seg4];

        draw_seg.extend(segments.iter());
        let result = split_all_paths(segments.clone());

        let moved_result: Vec<PathSegment> = result
            .iter()
            .enumerate()
            .map(|(i, seg)| seg.move_to(Point::from_xy(0.0, 5.0)))
            .collect();

        draw_seg.extend(moved_result.iter());

        path_segments_to_image(draw_seg, vec![]);
        assert_eq!(result.len(), 12);
    }

    #[derive(Debug)]
    struct MyPathBuilder {
        builder: Option<PathBuilder>,
        paths: Vec<Path>,
    }

    impl MyPathBuilder {
        fn new() -> Self {
            Self {
                builder: Some(PathBuilder::new()),
                paths: Vec::new(),
            }
        }

        fn paths(self) -> Vec<Path> {
            self.paths
        }
    }

    // font は y 軸の向きが逆
    impl OutlineBuilder for MyPathBuilder {
        fn move_to(&mut self, x: f32, y: f32) {
            println!("move to");
            self.builder.as_mut().unwrap().move_to(x, -y);
        }

        fn line_to(&mut self, x: f32, y: f32) {
            println!("line to");
            self.builder.as_mut().unwrap().line_to(x, -y);
        }

        fn quad_to(&mut self, x: f32, y: f32, x1: f32, y1: f32) {
            println!("quad to");
            self.builder.as_mut().unwrap().quad_to(x1, -y1, x, -y);
        }

        fn curve_to(&mut self, x: f32, y: f32, x1: f32, y1: f32, x2: f32, y2: f32) {
            println!("curve to");
            self.builder
                .as_mut()
                .unwrap()
                .cubic_to(x1, -y1, x2, -y2, x, -y);
        }

        fn close(&mut self) {
            println!("close");
            let mut builder = self.builder.replace(PathBuilder::new()).unwrap();
            builder.close();
            self.paths.push(builder.finish().unwrap());
        }
    }

    // segments と dots が Pixmap の中に納まるような transform を計算する
    fn calc_transform(
        canvas_size: f32,
        segments: &Vec<&PathSegment>,
        dots: &Vec<&Point>,
    ) -> (Transform, f32) {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for segment in segments {
            match segment {
                PathSegment::Line(Line { from, to }) => {
                    min_x = min_x.min(from.x).min(to.x);
                    min_y = min_y.min(from.y).min(to.y);
                    max_x = max_x.max(from.x).max(to.x);
                    max_y = max_y.max(from.y).max(to.y);
                }
                PathSegment::Quadratic(Quadratic { from, to, control }) => {
                    min_x = min_x.min(from.x).min(to.x).min(control.x);
                    min_y = min_y.min(from.y).min(to.y).min(control.y);
                    max_x = max_x.max(from.x).max(to.x).max(control.x);
                    max_y = max_y.max(from.y).max(to.y).max(control.y);
                }
                PathSegment::Cubic(Cubic {
                    from,
                    to,
                    control1,
                    control2,
                }) => {
                    min_x = min_x.min(from.x).min(to.x).min(control1.x).min(control2.x);
                    min_y = min_y.min(from.y).min(to.y).min(control1.y).min(control2.y);
                    max_x = max_x.max(from.x).max(to.x).max(control1.x).max(control2.x);
                    max_y = max_y.max(from.y).max(to.y).max(control1.y).max(control2.y);
                }
            }
        }

        for dot in dots {
            min_x = min_x.min(dot.x);
            min_y = min_y.min(dot.y);
            max_x = max_x.max(dot.x);
            max_y = max_y.max(dot.y);
        }

        let width = max_x - min_x;
        let height = max_y - min_y;
        let scale = canvas_size / width.max(height);

        let translate_x = -min_x * scale;
        let translate_y = -min_y * scale;

        (
            Transform::identity()
                .post_scale(scale, scale)
                .post_translate(translate_x, translate_y),
            scale,
        )
    }

    fn path_segments_to_image(segments: Vec<&PathSegment>, dots: Vec<&Point>) {
        let canvas_size = 500.0;
        let (transform, scale) = calc_transform(canvas_size, &segments, &dots);
        let scale_unit = 1.0 / scale;
        println!("scale: {}, scale_unit: {}", scale, scale_unit);

        let mut paint = Paint::default();
        let mut pixmap = Pixmap::new(canvas_size as u32, canvas_size as u32).unwrap();
        let mut stroke = Stroke::default();
        stroke.width = scale_unit;
        paint.anti_alias = true;

        let mut dot_stroke = Stroke::default();
        dot_stroke.width = scale_unit * 5.0;
        dot_stroke.line_cap = tiny_skia::LineCap::Round;

        for segment in segments {
            let (from, to) = segment.endpoints();
            let from_dot = {
                let mut from_dot = PathBuilder::new();
                from_dot.move_to(from.x, from.y);
                from_dot.line_to(from.x + f32::EPSILON, from.y + f32::EPSILON);
                from_dot.finish().unwrap()
            };
            let to_dot = {
                let mut to_dot = PathBuilder::new();
                to_dot.move_to(to.x, to.y);
                to_dot.line_to(to.x + f32::EPSILON, to.y + f32::EPSILON);
                to_dot.finish().unwrap()
            };

            let path = {
                let mut pb = PathBuilder::new();
                match segment {
                    PathSegment::Line(Line { from, to }) => {
                        pb.move_to(from.x, from.y);
                        pb.line_to(to.x, to.y);
                    }
                    PathSegment::Quadratic(Quadratic { from, to, control }) => {
                        pb.move_to(from.x, from.y);
                        pb.quad_to(control.x, control.y, to.x, to.y);
                    }
                    PathSegment::Cubic(Cubic {
                        from,
                        to,
                        control1,
                        control2,
                    }) => {
                        pb.move_to(from.x, from.y);
                        pb.cubic_to(control1.x, control1.y, control2.x, control2.y, to.x, to.y);
                    }
                }
                pb.finish().unwrap()
            };

            paint.set_color_rgba8(0, 127, 0, 255);
            pixmap.stroke_path(&path, &paint, &stroke, transform, None);
            paint.set_color_rgba8(255, 0, 0, 120);
            pixmap.stroke_path(&from_dot, &paint, &dot_stroke, transform, None);
            paint.set_color_rgba8(0, 255, 0, 120);
            pixmap.stroke_path(&to_dot, &paint, &dot_stroke, transform, None);
        }

        paint.set_color_rgba8(0, 0, 255, 255);
        for dot in dots {
            let mut dot_path = PathBuilder::new();
            dot_path.move_to(dot.x, dot.y);
            dot_path.line_to(dot.x + f32::EPSILON, dot.y + f32::EPSILON);
            let dot_path = dot_path.finish().unwrap();
            pixmap.stroke_path(&dot_path, &paint, &dot_stroke, transform, None);
        }

        pixmap.save_png("image.png").unwrap();
    }

    #[test]
    fn test_closs_point_line_intersect() {
        let line1 = Line {
            from: Point::from_xy(0.0, 0.0),
            to: Point::from_xy(2.0, 2.0),
        };
        let line2 = Line {
            from: Point::from_xy(1.0, 2.0),
            to: Point::from_xy(3.0, 0.0),
        };

        let result = cross_point_line(&line1, &line2);

        assert!(result.is_some());
        let cross_point = result.unwrap();
        path_segments_to_image(
            vec![&PathSegment::Line(line1), &PathSegment::Line(line2)],
            vec![&cross_point.point],
        );

        assert!((cross_point.point.x - 1.5).abs() < EPSILON);
        assert!((cross_point.point.y - 1.5).abs() < EPSILON);
        assert!((cross_point.a_position - 0.75).abs() < EPSILON);
        assert!((cross_point.b_position - 0.25).abs() < EPSILON);
    }

    #[test]
    fn test_closs_point_line_parallel() {
        let line1 = Line {
            from: Point::from_xy(0.0, 0.0),
            to: Point::from_xy(2.0, 2.0),
        };
        let line2 = Line {
            from: Point::from_xy(0.0, 1.0),
            to: Point::from_xy(2.0, 3.0),
        };

        let result = cross_point_line(&line1, &line2);
        assert!(result.is_none());
    }

    #[test]
    fn test_closs_point_line_no_intersect() {
        let line1 = Line {
            from: Point::from_xy(0.0, 0.0),
            to: Point::from_xy(1.0, 1.0),
        };
        let line2 = Line {
            from: Point::from_xy(2.0, 2.0),
            to: Point::from_xy(3.0, 3.0),
        };

        let result = cross_point_line(&line1, &line2);
        assert!(result.is_none());
    }
}
