use tiny_skia::Point;

enum PathSegment {
    Line {
        from: Point,
        to: Point,
    },
    Quadratic {
        from: Point,
        to: Point,
        control: Point,
    },
    Cubic {
        from: Point,
        to: Point,
        control1: Point,
        control2: Point,
    },
}

fn cross_point(a: PathSegment, b: PathSegment) -> Vec<Point> {
    match (a, b) {
        (PathSegment::Line { from: p1, to: p2 }, PathSegment::Line { from: p3, to: p4 }) => {
            // 直線同士の交点を求める
            let denom = (p4.y - p3.y) * (p2.x - p1.x) - (p4.x - p3.x) * (p2.y - p1.y);
            if denom == 0.0 {
                return vec![]; // 平行な場合は交点なし
            }
            let ua = ((p4.x - p3.x) * (p1.y - p3.y) - (p4.y - p3.y) * (p1.x - p3.x)) / denom;
            let ub = ((p2.x - p1.x) * (p1.y - p3.y) - (p2.y - p1.y) * (p1.x - p3.x)) / denom;
            if ua >= 0.0 && ua <= 1.0 && ub >= 0.0 && ub <= 1.0 {
                let x = p1.x + ua * (p2.x - p1.x);
                let y = p1.y + ua * (p2.y - p1.y);
                vec![Point::from_xy(x, y)]
            } else {
                vec![] // 線分上に交点がない場合
            }
        }
        (PathSegment::Line { from: p1, to: p2 }, PathSegment::Quadratic { from: q1, to: q2, control }) |
        (PathSegment::Quadratic { from: q1, to: q2, control }, PathSegment::Line { from: p1, to: p2 }) => {
            // 直線と二次ベジェ曲線の交点を近似して求める
            let mut points = Vec::new();
            let steps = 100; // 近似のためのステップ数
            for i in 0..steps {
                let t1 = i as f32 / steps as f32;
                let t2 = (i + 1) as f32 / steps as f32;
                let q_start = quadratic_bezier(q1, control, q2, t1);
                let q_end = quadratic_bezier(q1, control, q2, t2);
                let segment = PathSegment::Line { from: q_start, to: q_end };
                points.extend(cross_point(PathSegment::Line { from: p1, to: p2 }, segment));
            }
            points
        }
        // 他のセグメントの組み合わせについては未実装
        _ => vec![],
    }
}

fn quadratic_bezier(p0: Point, p1: Point, p2: Point, t: f32) -> Point {
    let x = (1.0 - t).powi(2) * p0.x + 2.0 * (1.0 - t) * t * p1.x + t.powi(2) * p2.x;
    let y = (1.0 - t).powi(2) * p0.y + 2.0 * (1.0 - t) * t * p1.y + t.powi(2) * p2.y;
    Point::from_xy(x, y)
}

#[cfg(test)]
mod tests {
    use tiny_skia::Point;

    use crate::{cross_point, PathSegment};

    #[test]
    fn test_cross_point_lines_intersect() {
        let p1 = Point::from_xy(0.0, 0.0);
        let p2 = Point::from_xy(2.0, 2.0);
        let p3 = Point::from_xy(0.0, 2.0);
        let p4 = Point::from_xy(2.0, 0.0);

        let segment1 = PathSegment::Line { from: p1, to: p2 };
        let segment2 = PathSegment::Line { from: p3, to: p4 };

        let result = cross_point(segment1, segment2);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], Point::from_xy(1.0, 1.0));
    }

    #[test]
    fn test_cross_point_lines_parallel() {
        let p1 = Point::from_xy(0.0, 0.0);
        let p2 = Point::from_xy(2.0, 2.0);
        let p3 = Point::from_xy(0.0, 1.0);
        let p4 = Point::from_xy(2.0, 3.0);

        let segment1 = PathSegment::Line { from: p1, to: p2 };
        let segment2 = PathSegment::Line { from: p3, to: p4 };

        let result = cross_point(segment1, segment2);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_cross_point_lines_no_intersect() {
        let p1 = Point::from_xy(0.0, 0.0);
        let p2 = Point::from_xy(1.0, 1.0);
        let p3 = Point::from_xy(2.0, 2.0);
        let p4 = Point::from_xy(3.0, 3.0);

        let segment1 = PathSegment::Line { from: p1, to: p2 };
        let segment2 = PathSegment::Line { from: p3, to: p4 };

        let result = cross_point(segment1, segment2);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_cross_point_line_quadratic() {
        let p1 = Point::from_xy(0.0, 0.0);
        let p2 = Point::from_xy(2.0, 2.0);
        let q1 = Point::from_xy(0.0, 2.0);
        let q2 = Point::from_xy(2.0, 0.0);
        let control = Point::from_xy(1.0, 3.0);

        let segment1 = PathSegment::Line { from: p1, to: p2 };
        let segment2 = PathSegment::Quadratic { from: q1, to: q2, control };

        let result = cross_point(segment1, segment2);
        assert!(!result.is_empty());
    }
}
