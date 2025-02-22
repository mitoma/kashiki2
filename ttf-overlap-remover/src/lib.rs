use tiny_skia_path::{path_geometry, NormalizedF32Exclusive, Path, Point, Rect};

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

trait SegmentTrait
where
    Self: Sized + Clone,
{
    fn endpoints(&self) -> (Point, Point);
    fn rect(&self) -> Rect;
    fn chop_harf(&self) -> (Self, Self);
    fn chop(&self, position: f32) -> (Self, Self);
    fn to_path_segment(self) -> PathSegment;
}

#[derive(Clone)]
struct Line {
    from: Point,
    to: Point,
}

impl SegmentTrait for Line {
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

#[derive(Clone)]

struct Quadratic {
    from: Point,
    to: Point,
    control: Point,
}

impl SegmentTrait for Quadratic {
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

#[derive(Clone)]
struct Cubic {
    from: Point,
    to: Point,
    control1: Point,
    control2: Point,
}

impl SegmentTrait for Cubic {
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

enum PathSegment {
    Line(Line),
    Quadratic(Quadratic),
    Cubic(Cubic),
}

impl PathSegment {
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

const EPSILON: f32 = 0.001;
fn cross_point(a: &PathSegment, b: &PathSegment) -> Vec<Point> {
    // 二つのセグメントが交差しているかどうかを矩形で判定
    if a.rect().intersect(&b.rect()).is_none() {
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
            if 0.0 < ua && ua < 1.0 && 0.0 < ub && ub < 1.0 {
                let x = p1.x + ua * (p2.x - p1.x);
                let y = p1.y + ua * (p2.y - p1.y);
                vec![Point::from_xy(x, y)]
            } else {
                vec![] // 線分上に交点がない場合
            }
        }
        (PathSegment::Line(line), PathSegment::Quadratic(quad))
        | (PathSegment::Quadratic(quad), PathSegment::Line(line)) => closs_point_inner(line, quad),
        (PathSegment::Line(line), PathSegment::Cubic(cubic))
        | (PathSegment::Cubic(cubic), PathSegment::Line(line)) => closs_point_inner(line, cubic),
        (PathSegment::Quadratic(quadratic), PathSegment::Cubic(cubic))
        | (PathSegment::Cubic(cubic), PathSegment::Quadratic(quadratic)) => {
            closs_point_inner(quadratic, cubic)
        }
        (PathSegment::Quadratic(quadratic1), PathSegment::Quadratic(quadratic2)) => {
            closs_point_inner(quadratic1, quadratic2)
        }
        (PathSegment::Cubic(cubic1), PathSegment::Cubic(cubic2)) => {
            closs_point_inner(cubic1, cubic2)
        }
    }
}

fn closs_point_inner<T: SegmentTrait, U: SegmentTrait>(a: &T, b: &U) -> Vec<Point> {
    let mut stack: Vec<(T, U)> = vec![(a.clone(), b.clone())];
    let mut points = Vec::new();

    while let Some((a, b)) = stack.pop() {
        let intersect = a.rect().intersect(&b.rect());
        if let Some(intersect) = intersect {
            // 幅もしくは高さが 0 の場合は交点なし
            /*
            if intersect.width().is_nearly_zero() || intersect.height().is_nearly_zero() {
                println!("no intersect");
                continue;
            }
             */
            if intersect.width() < EPSILON && intersect.height() < EPSILON {
                let (a_from, a_to) = a.endpoints();
                let (b_from, b_to) = b.endpoints();
                points.append(&mut cross_point(
                    &PathSegment::Line(Line {
                        from: a_from,
                        to: a_to,
                    }),
                    &PathSegment::Line(Line {
                        from: b_from,
                        to: b_to,
                    }),
                ));
            } else {
                let (a1, a2) = a.chop_harf();
                let (b1, b2) = b.chop_harf();
                stack.push((a1.clone(), b1.clone()));
                stack.push((a2.clone(), b2.clone()));
                stack.push((a1, b2));
                stack.push((a2, b1));
            }
        }
    }

    points.dedup();
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

    use crate::{cross_point, path_to_path_segments, Cubic, Line, PathSegment, Quadratic};

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
        assert_eq!(result[0], Point::from_xy(1.0, 1.0));
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
        assert!((result[0].x - intersection.x).abs() < 0.01);
        assert!((result[0].y - intersection.y).abs() < 0.01);
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
        path_segments_to_image(vec![&segment1, &segment2], result.iter().collect());
        assert_eq!(result.len(), 1);
        let intersection = Point::from_xy(0.5, 1.0);
        assert!((result[0] - intersection).x < 0.01);
        assert!((result[0] - intersection).y < 0.01);
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
        path_segments_to_image(vec![&line_seg, &cubic_seg], result.iter().collect());
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
        path_segments_to_image(vec![&cubic_seg1, &cubic_seg2], result.iter().collect());
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
        path_segments_to_image(segments.iter().collect(), dots.iter().collect());
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
}
