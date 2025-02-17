use tiny_skia_path::{path_geometry, NormalizedF32Exclusive, Point, Rect};

trait SegmentTrait
where
    Self: Sized,
{
    fn rect(&self) -> Rect;
    fn chop_harf(&self) -> (Self, Self);
    fn chop(&self, position: f32) -> (Self, Self);
    fn to_path_segment(self) -> PathSegment;
}

struct Line {
    from: Point,
    to: Point,
}

impl SegmentTrait for Line {
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

struct Quadratic {
    from: Point,
    to: Point,
    control: Point,
}

impl SegmentTrait for Quadratic {
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
        let _ = path_geometry::chop_quad_at(&arg, center, &mut result);
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
struct Cubic {
    from: Point,
    to: Point,
    control1: Point,
    control2: Point,
}

impl SegmentTrait for Cubic {
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
        let _ = path_geometry::chop_cubic_at2(&arg, center, &mut result);
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

enum PathSegment {
    Line(Line),
    Quadratic(Quadratic),
    Cubic(Cubic),
}

impl PathSegment {
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

const EPSILON: f32 = 0.0001;
fn cross_point(a: PathSegment, b: PathSegment) -> Vec<Point> {
    // 二つのセグメントが交差しているかどうかを矩形で判定
    let a_rect = a.rect();
    let b_rect = b.rect();
    let Some(intersect) = a_rect.intersect(&b_rect) else {
        return vec![];
    };
    match (a, b) {
        (
            PathSegment::Line(Line { from: p1, to: p2 }),
            PathSegment::Line(Line { from: p3, to: p4 }),
        ) => {
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
        (PathSegment::Line(line), PathSegment::Quadratic(quad))
        | (PathSegment::Quadratic(quad), PathSegment::Line(line)) => {
            if intersect.width() < EPSILON && intersect.height() < EPSILON {
                // 重なる矩形が十分小さい場合は二次ベジェ曲線は直線とみなして交点を求める
                return cross_point(
                    PathSegment::Line(Line {
                        from: line.from,
                        to: line.to,
                    }),
                    PathSegment::Line(Line {
                        from: quad.from,
                        to: quad.to,
                    }),
                );
            }
            // 直線と二次ベジェ曲線の交点を近似して求める
            let mut points = Vec::new();

            let (line1, line2) = line.chop_harf();
            let (quad1, quad2) = quad.chop_harf();
            points.append(&mut cross_point(
                line1.to_path_segment(),
                quad1.to_path_segment(),
            ));
            points.append(&mut cross_point(
                line2.to_path_segment(),
                quad2.to_path_segment(),
            ));
            let (line1, line2) = line.chop_harf();
            let (quad1, quad2) = quad.chop_harf();
            points.append(&mut cross_point(
                line1.to_path_segment(),
                quad2.to_path_segment(),
            ));
            points.append(&mut cross_point(
                line2.to_path_segment(),
                quad1.to_path_segment(),
            ));
            points.dedup();
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
    use std::vec;

    use tiny_skia::{Paint, PathBuilder, Pixmap, Point, Stroke, Transform};
    use tiny_skia_path::{path_geometry, NormalizedF32Exclusive};

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
        let segment2 = PathSegment::Quadratic {
            from: q1,
            to: q2,
            control,
        };

        let result = cross_point(segment1, segment2);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_cross_point_line_quadratic_one_intersection() {
        let p1 = Point::from_xy(0.0, 0.0);
        let p2 = Point::from_xy(2.0, 2.0);
        let q1 = Point::from_xy(0.0, 2.0);
        let q2 = Point::from_xy(2.0, 0.0);
        let control = Point::from_xy(1.0, 1.0);

        let segment1 = PathSegment::Line { from: p1, to: p2 };
        let segment2 = PathSegment::Quadratic {
            from: q1,
            to: q2,
            control,
        };

        path_segments_to_image(vec![&segment1, &segment2]);

        let result = cross_point(segment1, segment2);
        println!("{:?}", result);
        assert_eq!(result.len(), 1);
        let intersection = Point::from_xy(1.0, 1.0);
        assert!((result[0].x - intersection.x).abs() < 0.01);
        assert!((result[0].y - intersection.y).abs() < 0.01);
    }

    #[test]
    fn test_cross_point_line_quadratic_two_intersections() {
        let p1 = Point::from_xy(0.0, 1.0);
        let p2 = Point::from_xy(2.0, 1.0);
        let q1 = Point::from_xy(0.0, 0.0);
        let q2 = Point::from_xy(2.0, 2.0);
        let control = Point::from_xy(1.0, 3.0);

        let segment1 = PathSegment::Line { from: p1, to: p2 };
        let segment2 = PathSegment::Quadratic {
            from: q1,
            to: q2,
            control,
        };

        path_segments_to_image(vec![&segment1, &segment2]);

        let result = cross_point(segment1, segment2);
        assert_eq!(result.len(), 1);
        let intersection = Point::from_xy(0.5, 1.0);
        assert!((result[0] - intersection).x < 0.01);
        assert!((result[0] - intersection).y < 0.01);
    }

    #[test]
    fn test_chop() {
        let q1 = Point::from_xy(0.0, 0.0);
        let q2 = Point::from_xy(2.0, 2.0);
        let control = Point::from_xy(1.0, 3.0);

        let segment = PathSegment::Quadratic {
            from: q1,
            to: q2,
            control,
        };

        let arg = [q1, control, q2];
        let mut result: [Point; 5] = Default::default();
        let center = NormalizedF32Exclusive::new_bounded(0.5);

        let _ = path_geometry::chop_quad_at(&arg, center, &mut result);
        let pre = PathSegment::Quadratic {
            from: result[0],
            to: result[2],
            control: result[1],
        };
        let post = PathSegment::Quadratic {
            from: result[2],
            to: result[4],
            control: result[3],
        };

        path_segments_to_image(vec![&segment, &pre, &post]);
    }

    #[test]
    fn test_chop2() {
        let p1 = Point::from_xy(1.0, 0.0);
        let p2 = Point::from_xy(0.0, 2.0);
        let line_seg = PathSegment::Line { from: p1, to: p2 };
        let (line1, line2) = line_seg.chop(0.3);

        let q1 = Point::from_xy(0.0, 0.0);
        let q2 = Point::from_xy(2.0, 2.0);
        let control = Point::from_xy(1.0, 3.0);

        let quad_seg = PathSegment::Quadratic {
            from: q1,
            to: q2,
            control,
        };

        let (quad1, quad2) = quad_seg.chop(0.5);

        path_segments_to_image(vec![&line1, &line2, &quad1, &quad2]);
    }

    #[test]
    fn test_cubic() {
        let p1 = Point::from_xy(0.0, 1.0);
        let p2 = Point::from_xy(2.0, 1.0);
        let c1 = Point::from_xy(0.5, 0.0);
        let c2 = Point::from_xy(1.7, 2.0);
        let line_seg = PathSegment::Cubic {
            from: p1,
            to: p2,
            control1: c1,
            control2: c2,
        };
        let (line1, line2) = line_seg.chop(0.3);
        path_segments_to_image(vec![&line1, &line2]);
    }

    fn path_segments_to_image(segments: Vec<&PathSegment>) {
        let mut paint = Paint::default();
        let mut pixmap = Pixmap::new(500, 500).unwrap();
        let mut stroke = Stroke::default();
        stroke.width = 0.01;
        paint.set_color_rgba8(0, 127, 0, 255);
        paint.anti_alias = true;

        for segment in segments {
            let path = {
                let mut pb = PathBuilder::new();
                match segment {
                    PathSegment::Line { from, to } => {
                        pb.move_to(from.x, from.y);
                        pb.line_to(to.x, to.y);
                    }
                    PathSegment::Quadratic { from, to, control } => {
                        pb.move_to(from.x, from.y);
                        pb.quad_to(control.x, control.y, to.x, to.y);
                    }
                    PathSegment::Cubic {
                        from,
                        to,
                        control1,
                        control2,
                    } => {
                        pb.move_to(from.x, from.y);
                        pb.cubic_to(control1.x, control1.y, control2.x, control2.y, to.x, to.y);
                    }
                }
                pb.finish().unwrap()
            };
            pixmap.stroke_path(
                &path,
                &paint,
                &stroke,
                Transform::identity()
                    .pre_translate(1.0, 1.0)
                    .post_scale(100.0, 100.0),
                None,
            );
        }
        pixmap.save_png("image.png").unwrap();
    }
}
