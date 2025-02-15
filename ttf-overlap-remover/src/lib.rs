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
        // 他のセグメントの組み合わせについては未実装
        _ => vec![],
    }
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
}
