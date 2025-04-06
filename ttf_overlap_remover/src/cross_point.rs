use tiny_skia_path::{Point, Rect};

use crate::{Line, PathSegment, SegmentTrait, sat::is_polygon_overlapping};

/// 二つのセグメントが交差しているかを判定して交差している場合はその交差点で二つのセグメントをそれぞれ分割する
///
/// 分割されなかった場合は None を返す。
/// 分割される場合はそれぞれのセグメントの Vec を返す。
pub(crate) fn split_line_on_cross_point(
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

    let mut a_sorted = cross_points
        .iter()
        .filter(|cp| ![0.0, 1.0].contains(&cp.a_position.abs()))
        .cloned()
        .collect::<Vec<_>>();
    a_sorted.sort_by(|l, r| l.a_position.partial_cmp(&r.a_position).unwrap());
    let a_result = if a_sorted.is_empty() {
        vec![a.clone()]
    } else {
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
                if !pre.same_from_to() {
                    // 端点が異なる場合は追加する
                    result.push(pre);
                }
                (result, post, consumed + cp.a_position)
            },
        );
        if !last.same_from_to() {
            // 端点が異なる場合は追加する
            a_result.push(last);
        }
        a_result
    };

    let mut b_sorted = cross_points
        .iter()
        .filter(|cp| ![0.0, 1.0].contains(&cp.b_position.abs()))
        .cloned()
        .collect::<Vec<_>>();
    b_sorted.sort_by(|l, r| l.b_position.partial_cmp(&r.b_position).unwrap());
    let b_result = if b_sorted.is_empty() {
        vec![b.clone()]
    } else {
        let (mut b_result, last, _) = b_sorted.iter().fold(
            (vec![], b.clone(), 0.0f32),
            |(mut result, target_path, consumed), cp| {
                let length = 1.0 - consumed;
                let next_gain = cp.b_position - consumed;
                let chop_point = next_gain / length;
                let (mut pre, mut post) = target_path.chop(chop_point);
                pre.set_to(cp.point);
                post.set_from(cp.point);
                if !pre.same_from_to() {
                    // 端点が異なる場合は追加する
                    result.push(pre);
                }
                (result, post, consumed + cp.b_position)
            },
        );
        if !last.same_from_to() {
            // 端点が異なる場合は追加する
            b_result.push(last);
        }
        b_result
    };
    if a_result.len() == 1 && b_result.len() == 1 {
        return None; // 分割されなかった場合は None を返す
    }
    Some((a_result, b_result))
}

const EPSILON: f32 = 0.000001;
fn cross_point(a: &PathSegment, b: &PathSegment) -> Vec<CrossPoint> {
    // 二つのセグメントが交差しているかどうかを判定
    if !is_polygon_overlapping(&a.polygon(), &b.polygon()) {
        return vec![];
    };

    match (a, b) {
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
    }
}

#[derive(Debug, Clone, PartialEq)]
struct CrossPoint {
    point: Point,
    // 交点が線分のどの位置にあるかを示す。0.0 から 1.0 の範囲で示す
    a_position: f32,
    b_position: f32,
}

impl CrossPoint {
    fn normalize(&self) -> CrossPoint {
        CrossPoint {
            point: self.point,
            a_position: Self::position_normalize(self.a_position),
            b_position: Self::position_normalize(self.b_position),
        }
    }

    #[inline]
    fn position_normalize(value: f32) -> f32 {
        if 0.0 < value && value < EPSILON {
            0.0
        } else if 1.0 - EPSILON < value && value < 1.0 {
            1.0
        } else {
            value
        }
    }
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
    if (0.0..=1.0).contains(&ua) && (0.0..=1.0).contains(&ub) {
        let x = a.from.x + ua * (a.to.x - a.from.x);
        let y = a.from.y + ua * (a.to.y - a.from.y);
        Some(
            CrossPoint {
                point: Point::from_xy(x, y),
                a_position: ua,
                b_position: ub,
            }
            .normalize(),
        )
    } else {
        None // 線分上に交点がない場合
    }
}

#[inline]
fn closs_point_inner<T, U>(a: &T, b: &U) -> Vec<CrossPoint>
where
    T: SegmentTrait + std::fmt::Debug,
    U: SegmentTrait + std::fmt::Debug,
{
    struct StackItem<T, U> {
        a: T,
        a_position: f32,
        a_depth: u32,
        b: U,
        b_position: f32,
        b_depth: u32,
    }

    let mut stack: Vec<StackItem<T, U>> = vec![StackItem {
        a: a.clone(),
        a_position: 0.0,
        a_depth: 0,
        b: b.clone(),
        b_position: 0.0,
        b_depth: 0,
    }];
    let mut points = Vec::new();

    // 端点が交点となる場合は先に交点として追加しておく
    if a.endpoints().0 == b.endpoints().0 {
        points.push(CrossPoint {
            point: a.endpoints().0,
            a_position: 0.0,
            b_position: 0.0,
        });
    }
    if a.endpoints().0 == b.endpoints().1 {
        points.push(CrossPoint {
            point: a.endpoints().0,
            a_position: 0.0,
            b_position: 1.0,
        });
    }
    if a.endpoints().1 == b.endpoints().0 {
        points.push(CrossPoint {
            point: a.endpoints().1,
            a_position: 1.0,
            b_position: 0.0,
        });
    }
    if a.endpoints().1 == b.endpoints().1 {
        points.push(CrossPoint {
            point: a.endpoints().1,
            a_position: 1.0,
            b_position: 1.0,
        });
    }

    while let Some(StackItem {
        a,
        a_position,
        a_depth,
        b,
        b_position,
        b_depth,
    }) = stack.pop()
    {
        let intersect = a.rect().intersect(&b.rect());
        if let Some(intersect) = intersect {
            if is_small_rect(&intersect) || a_depth > 8 || b_depth > 8 {
                let a_gain = 1.0 / (2u32.pow(a_depth) as f32);
                let b_gain = 1.0 / (2u32.pow(b_depth) as f32);
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
                    // 交点が線分の端点に近い場合は端点として扱う
                    let cp = CrossPoint {
                        point: point.point,
                        a_position: a_position + point.a_position * a_gain,
                        b_position: b_position + point.b_position * b_gain,
                    }
                    .normalize();
                    // 交点が端点に丸められている際に、既に端点で既に交点が追加されているばあいは追加しない
                    // point は厳密に一致しない可能性が高いので、a_position と b_position で判定する
                    if !points
                        .iter()
                        .any(|p| p.a_position == cp.a_position && p.b_position == cp.b_position)
                    {
                        points.push(cp)
                    }
                }
            } else {
                let a_depth = a_depth + 1;
                let b_depth = b_depth + 1;
                let a_gain = 1.0 / (2u32.pow(a_depth) as f32);
                let b_gain = 1.0 / (2u32.pow(b_depth) as f32);
                let (a1, a2) = a.chop_harf();
                let (b1, b2) = b.chop_harf();
                stack.push(StackItem {
                    a: a1.clone(),
                    a_position,
                    a_depth,
                    b: b1.clone(),
                    b_position,
                    b_depth,
                });
                stack.push(StackItem {
                    a: a1.clone(),
                    a_position,
                    a_depth,
                    b: b2.clone(),
                    b_position: b_position + b_gain,
                    b_depth,
                });
                stack.push(StackItem {
                    a: a2.clone(),
                    a_position: a_position + a_gain,
                    a_depth,
                    b: b1.clone(),
                    b_position,
                    b_depth,
                });
                stack.push(StackItem {
                    a: a2.clone(),
                    a_position: a_position + a_gain,
                    a_depth,
                    b: b2.clone(),
                    b_position: b_position + b_gain,
                    b_depth,
                });
            }
        }
    }
    points
}

#[inline]
fn is_small_rect(rect: &Rect) -> bool {
    rect.width() < EPSILON && rect.height() < EPSILON
}

#[cfg(test)]
mod tests {

    use tiny_skia_path::{NormalizedF32Exclusive, Point, path_geometry};

    use crate::{
        Cubic, Line, PathSegment, Quadratic,
        cross_point::{CrossPoint, EPSILON, cross_point, cross_point_line},
        split_all_paths, split_line_on_cross_point,
        test_helper::{path_segments_to_image, path_segments_to_images},
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
        assert_eq!(result.len(), 1);
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

        path_geometry::chop_quad_at(&arg, center, &mut result);
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
    fn test_split_dog_quad() {
        // 🐕の絵文字で分割ミスが発生するのを再現するテストケース
        let quad_seg1 = PathSegment::Quadratic(Quadratic {
            from: Point::from_xy(1384.5, -1549.0),
            to: Point::from_xy(1330.0, -1617.0),
            control: Point::from_xy(1360.0, -1598.0),
        });
        let quad_seg2 = PathSegment::Quadratic(Quadratic {
            from: Point::from_xy(1512.0, -1431.0),
            to: Point::from_xy(1334.0, -1600.0),
            control: Point::from_xy(1449.0, -1540.0),
        });

        //let quad_seg1 = quad_seg1.chop(0.5).0.chop(0.5).1.chop(0.5).1; //.chop(0.5).0;
        //let quad_seg2 = quad_seg2.chop(0.5).1.chop(0.5).1.chop(0.5).0; //.chop(0.5).1;

        println!("{:?}", quad_seg1);
        println!("{:?}", quad_seg2);
        let cross_point = cross_point(&quad_seg1, &quad_seg2);
        let points = cross_point.iter().map(|cp| &cp.point).collect::<Vec<_>>();

        path_segments_to_image(vec![&quad_seg1, &quad_seg2], points);

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

        assert_eq!(split1.len(), 2);
        assert_eq!(split2.len(), 2);
    }

    #[test]
    fn test_split_uni_quad() {
        // 🦄の絵文字で分割ミスが発生するのを再現するテストケース
        // 分割後のセグメントがほぼ同じセグメントになっていて無限ループを誘発している
        //i:Quadratic(Quadratic { from: Point { x: 1453.772, y: 1382.0125 }, to: Point { x: 1453.7717, y: 1382.0137 }, control: Point { x: 1450.3162, y: 1414.9514 } }),
        //j:Quadratic(Quadratic { from: Point { x: 1453.7714, y: 1382.0182 }, to: Point { x: 1453.7715, y: 1382.0171 }, control: Point { x: 1453.7715, y: 1382.0176 } })

        let quad_seg1 = PathSegment::Quadratic(Quadratic {
            from: Point::from_xy(1453.772, -1382.0125),
            to: Point::from_xy(1453.7717, -1382.0137),
            control: Point::from_xy(1450.3162, -1414.9514),
        });
        let quad_seg2 = PathSegment::Quadratic(Quadratic {
            from: Point::from_xy(1453.7714, -1382.0182),
            to: Point::from_xy(1453.7715, -1382.0171),
            control: Point::from_xy(1453.7715, -1382.0176),
        });

        println!("{:?}", quad_seg1);
        println!("{:?}", quad_seg2);
        let cross_point = cross_point(&quad_seg1, &quad_seg2);
        let points = cross_point.iter().map(|cp| &cp.point).collect::<Vec<_>>();

        path_segments_to_image(vec![&quad_seg1, &quad_seg2], points);

        let Some((_split1, _split2)) = split_line_on_cross_point(&quad_seg1, &quad_seg2) else {
            return;
        };
        unreachable!("分割はされないのが今の正解");
    }

    // FIXME できれば直したいが…
    //#[test]
    fn test_mogemoge() {
        //a:Quadratic(Quadratic { from: Point { x: 1172.0261, y: 423.0 }, to: Point { x: 1172.0, y: 425.0 }, control: Point { x: 1172.0, y: 423.99362 } }),
        //b:Line(Line { from: Point { x: 1172.0, y: 79.0 }, to: Point { x: 1172.0, y: 467.0 } })
        let quad_seg1 = PathSegment::Quadratic(Quadratic {
            from: Point::from_xy(1172.0261, 423.0),
            to: Point::from_xy(1172.0, 425.0),
            control: Point::from_xy(1172.0, 423.99362),
        });
        let quad_seg2 = PathSegment::Line(Line {
            from: Point::from_xy(1172.0, 79.0),
            to: Point::from_xy(1172.0, 467.0),
        });

        println!("{:?}", quad_seg1);
        println!("{:?}", quad_seg2);
        let cp = cross_point(&quad_seg1, &quad_seg2);
        println!("{:?}", cp);
        let points = cp.iter().map(|cp| &cp.point).collect::<Vec<_>>();

        path_segments_to_image(vec![&quad_seg1, &quad_seg2], points);

        let Some((a_result, b_result)) = split_line_on_cross_point(&quad_seg1, &quad_seg2) else {
            unreachable!("分割される");
        };

        for new_a in a_result.iter() {
            for new_b in b_result.iter() {
                let points = cross_point(new_a, new_b)
                    .into_iter()
                    // 端点同士が交点となる場合は分割対象外
                    .filter(|cp| {
                        !([0.0, 1.0].contains(&cp.a_position.abs())
                            && [0.0, 1.0].contains(&cp.b_position.abs()))
                    })
                    .collect::<Vec<_>>();
                if points.len() > 0 {
                    unreachable!("分割したのに分割後も交点が存在する");
                }
            }
        }
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
                let result = cross_point(result_seg[i], result_seg[j]);

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
            .map(|seg| seg.move_to(Point::from_xy(0.0, 3.0)))
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
            .map(|seg| seg.move_to(Point::from_xy(0.0, 5.0)))
            .collect();

        draw_seg.extend(moved_result.iter());

        path_segments_to_image(draw_seg, vec![]);
        assert_eq!(result.len(), 14);
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

    #[test]
    fn test_cross_point_line_quadratic2() {
        //path_i: Line(Line { from: Point { x: 1345.0, y: -990.9708 }, to: Point { x: 1345.0, y: -395.37598 } }),
        //path_j: Quadratic(Quadratic { from: Point { x: 1345.0, y: -990.9708 }, to: Point { x: 1320.0, y: -894.0 }, control: Point { x: 1342.2715, y: -933.8578 } })

        let p1 = Point::from_xy(1345.0, -990.9708);
        let p2 = Point::from_xy(1345.0, -395.37598);
        let q1 = Point::from_xy(1345.0, -990.9708);
        let q2 = Point::from_xy(1320.0, -894.0);
        let control = Point::from_xy(1342.2715, -933.8578);

        let segment1 = PathSegment::Line(Line { from: p1, to: p2 });
        let segment2 = PathSegment::Quadratic(Quadratic {
            from: q1,
            to: q2,
            control,
        });

        let result = cross_point(&segment1, &segment2);

        println!("{:?}", result);
        path_segments_to_images(
            "hogepoge",
            vec![&segment1, &segment2],
            result.iter().map(|cp| &cp.point).collect(),
        );
        assert!(!result.is_empty());
        assert_eq!(result.len(), 1);
        assert_eq!(
            result.first().unwrap(),
            &CrossPoint {
                point: Point::from_xy(1345.0, -990.9708),
                a_position: 0.0,
                b_position: 0.0,
            }
        );
    }
}
