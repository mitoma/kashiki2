use tiny_skia::Point;

enum PathSegment {
    Line { start: Point, end: Point },
    Quadratic(Point, Point, Point),
    Cubic(Point, Point, Point, Point),
}

fn cross_point(a: PathSegment, b: PathSegment) -> Vec<Point> {
    match (a, b) {
        (
            PathSegment::Line {
                start: p1_start,
                end: p1_end,
            },
            PathSegment::Line {
                start: p2_start,
                end: p2_end,
            },
        ) => {
            // 直線同士の交点を求める
            let denom = (p2_end.y - p2_start.y) * (p1_end.x - p1_start.x)
                - (p2_end.x - p2_start.x) * (p1_end.y - p1_start.y);
            if denom == 0.0 {
                return vec![]; // 平行な場合は交点なし
            }
            let ua = ((p2_end.x - p2_start.x) * (p1_start.y - p2_start.y)
                - (p2_end.y - p2_start.y) * (p1_start.x - p2_start.x))
                / denom;
            let ub = ((p1_end.x - p1_start.x) * (p1_start.y - p2_start.y)
                - (p1_end.y - p1_start.y) * (p1_start.x - p2_start.x))
                / denom;
            if ua >= 0.0 && ua <= 1.0 && ub >= 0.0 && ub <= 1.0 {
                let x = p1_start.x + ua * (p1_end.x - p1_start.x);
                let y = p1_start.y + ua * (p1_end.y - p1_start.y);
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

        let segment1 = PathSegment::Line { start: p1, end: p2 };
        let segment2 = PathSegment::Line { start: p3, end: p4 };

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

        let segment1 = PathSegment::Line { start: p1, end: p2 };
        let segment2 = PathSegment::Line { start: p3, end: p4 };

        let result = cross_point(segment1, segment2);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_cross_point_lines_no_intersect() {
        let p1 = Point::from_xy(0.0, 0.0);
        let p2 = Point::from_xy(1.0, 1.0);
        let p3 = Point::from_xy(2.0, 2.0);
        let p4 = Point::from_xy(3.0, 3.0);

        let segment1 = PathSegment::Line { start: p1, end: p2 };
        let segment2 = PathSegment::Line { start: p3, end: p4 };

        let result = cross_point(segment1, segment2);
        assert_eq!(result.len(), 0);
    }
}
