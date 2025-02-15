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
        (PathSegment::Line { .. }, PathSegment::Line { .. }) => {
            todo!()
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
