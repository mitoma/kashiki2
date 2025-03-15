use std::collections::HashMap;

use path_segment::{Cubic, Line, PathSegment, Quadratic, SegmentTrait};
use tiny_skia_path::{Path, Point};
use util::cmp_clockwise;

mod path_segment;
#[cfg(test)]
mod test_helper;
mod util;

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

/// Vec<Path> を PathSegment に変換する
fn paths_to_path_segments(paths: &Vec<Path>) -> Vec<PathSegment> {
    paths
        .iter()
        .flat_map(|path| path_to_path_segments(path.clone()))
        .collect()
}

#[inline]
fn is_closed(segments: &Vec<PathSegment>) -> bool {
    if segments.is_empty() {
        return false;
    }

    if !segments.windows(2).all(|segs| {
        let (_, first_to) = segs[0].endpoints();
        let (last_from, _) = segs[1].endpoints();
        first_to == last_from
    }) {
        return false;
    }

    let (first_from, _) = segments.first().unwrap().endpoints();
    let (_, last_to) = segments.last().unwrap().endpoints();
    first_from == last_to
}

fn is_clockwise(segments: &Vec<PathSegment>) -> bool {
    let mut sum = 0.0;
    for segment in segments {
        let (from, to) = segment.endpoints();
        //sum += from.cross(to);
        sum += from.x * to.y - from.y * to.x;
    }
    sum > 0.0
}

fn reverse(segments: &Vec<PathSegment>) -> Vec<PathSegment> {
    segments.iter().map(|s| s.reverse()).rev().collect()
}

fn same_path(segments1: &Vec<PathSegment>, segments2: &Vec<PathSegment>) -> bool {
    if segments1.len() != segments2.len() {
        return false;
    }
    let mut segment1_map: HashMap<String, usize> = HashMap::new();
    let mut segment2_map: HashMap<String, usize> = HashMap::new();
    for segment in segments1.iter() {
        *segment1_map.entry(format!("{:?}", segment)).or_insert(0) += 1;
    }
    for segment in segments2.iter() {
        *segment2_map.entry(format!("{:?}", segment)).or_insert(0) += 1;
    }
    segment1_map == segment2_map
}

pub fn remove_overlap(paths: Vec<Path>) -> Vec<Vec<PathSegment>> {
    // Path を全て PathFlagment に分解し、交差部分でセグメントを分割する
    let path_segments = paths
        .iter()
        .flat_map(|path| path_to_path_segments(path.clone()));
    let path_segments = split_all_paths(path_segments.collect());
    println!("最初のセグメント数: {:?}", path_segments.len());
    remove_overlap_inner(path_segments)
}

pub fn remove_overlap_rev(paths: Vec<Path>) -> Vec<Vec<PathSegment>> {
    // Path を全て PathFlagment に分解し、交差部分でセグメントを分割する
    let path_segments = paths
        .iter()
        .flat_map(|path| path_to_path_segments(path.clone()))
        //.map(|s| s.reverse())
        .rev();
    let path_segments = split_all_paths(path_segments.collect());
    println!("最初のセグメント数: {:?}", path_segments.len());
    remove_overlap_inner(path_segments)
}

fn get_loop_segment(path_segments: Vec<PathSegment>, clock_wise: bool) -> Vec<Vec<PathSegment>> {
    // 分解された PathFlagment からつなげてパスの候補となる Vec<PathSegment> を構成する
    let mut result_paths: Vec<Vec<PathSegment>> = Vec::new();

    for segment in path_segments.iter().flat_map(|p| [p.clone(), p.reverse()]) {
        // 既にパス候補に含まれているセグメントであればスキップ
        let mut current_segment = segment.clone();
        let mut current_path = Vec::new();
        current_path.push(current_segment.clone());
        loop {
            // 次のパスになりうるセグメントを探す(current の to が next の from または to と一致するセグメント)
            let mut nexts: Vec<PathSegment> = path_segments
                .iter()
                // 今のセグメントと同一または逆向きのセグメントは除外
                .filter(|s| !s.is_same_or_reversed(&current_segment))
                // 今のセグメントと繋がるパスを絞り込む
                .flat_map(|s| {
                    let (_, current_to) = current_segment.endpoints();
                    let (next_from, next_to) = s.endpoints();
                    if current_to == next_from {
                        Some(s.clone())
                    } else if current_to == next_to {
                        Some(s.reverse())
                    } else {
                        None
                    }
                })
                // 今のパスに含まれているセグメントと逆向きのセグメントは除外
                .filter(|s| {
                    let rev = s.reverse();
                    current_path.iter().all(|cs| cs != &rev)
                })
                .collect();
            if nexts.is_empty() {
                // 次のパスになりうるセグメントが見つからない場合、閉じていない Path だった可能性もあるのでまぁいいかという感じで次のセグメントに進む
                break;
            }

            // 現在のセグメントの進行方向から、最も左向きのベクトルを持つセグメントを次のセグメントとして選択する
            nexts.sort_by(|l, r| {
                let v1 = -current_segment.to_vector();
                let v2 = l.from_vector();
                let v3 = r.from_vector();
                cmp_clockwise(&v1, &v2, &v3)
            });
            if clock_wise {
                current_segment = nexts.first().unwrap().clone();
            } else {
                current_segment = nexts.last().unwrap().clone();
            }

            current_path.push(current_segment.clone());
            if let Some(loop_position) = has_vector_tail_loop(&current_path) {
                let created_path = current_path.split_off(loop_position);
                let path_start = created_path.first().unwrap().endpoints().0;
                let path_end = created_path.last().unwrap().endpoints().1;
                println!(
                    "パスの開始と終了の確認: {:?} {:?}, len:{}",
                    path_start,
                    path_end,
                    created_path.len()
                );

                let has_same_path = result_paths.iter().any(|s| same_path(&created_path, s));
                if has_same_path {
                    println!("同じパスが既に存在しているのでスキップ");
                    break;
                }
                result_paths.push(created_path);
                break;
            }
        }
    }
    result_paths
}

/// overlap が含まれる path を受け取り、overlap を除去した path を返す
pub fn remove_overlap_inner(path_segments: Vec<PathSegment>) -> Vec<Vec<PathSegment>> {
    // 分解された PathFlagment からつなげてパスの候補となる Vec<PathSegment> を構成する
    let result_paths = get_loop_segment(path_segments.clone(), false);

    // TODO おそらくここで、右回転のパスなのに関わらず左回転のパスと接しているパスを除外するとよい
    let mut clockwise: Vec<Vec<PathSegment>> = result_paths
        .iter()
        .cloned()
        .filter(|segments| is_clockwise(segments))
        .collect();
    let rev_clockwise: Vec<Vec<PathSegment>> = result_paths
        .iter()
        .cloned()
        .filter(|segments| !is_clockwise(segments))
        .collect();
    println!("時計回りのパス数: {:?}", clockwise.len());
    println!("反時計回りのパス数: {:?}", rev_clockwise.len());

    let clockwise_points = clockwise
        .iter()
        .flat_map(|segments| {
            segments
                .iter()
                .flat_map(|segment| {
                    let (f, t) = segment.endpoints();
                    [f]
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let rev_clockwise_points = rev_clockwise
        .iter()
        .flat_map(|segments| {
            segments
                .iter()
                .flat_map(|segment| {
                    let (f, t) = segment.endpoints();
                    [f]
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let mut filterd_clockwise: Vec<Vec<PathSegment>> = clockwise
        .iter()
        .cloned()
        .filter(|segments| {
            segments.iter().all(|segment| {
                let (f, t) = segment.endpoints();
                !rev_clockwise_points.contains(&f) && !rev_clockwise_points.contains(&t)
            })
        })
        .collect();
    let filterd_counter_clockwise: Vec<Vec<PathSegment>> = rev_clockwise
        .iter()
        .cloned()
        .filter(|segments| {
            segments.iter().all(|segment| {
                let (f, t) = segment.endpoints();
                !clockwise_points.contains(&f) && !clockwise_points.contains(&t)
            })
        })
        .collect();

    clockwise.extend(filterd_counter_clockwise);
    clockwise

    //filterd_clockwise.extend(filterd_counter_clockwise);
}

/// 末尾にループが発生している時にループの開始位置を返す関数。
fn has_vector_tail_loop<T: PartialEq>(value: &Vec<T>) -> Option<usize> {
    let len = value.len();
    for i in 1..len {
        if len < (1 + i) * 2 {
            continue;
        }
        if value[len - 1 - i..] == value[len - ((1 + i) * 2)..(len - (1 + i))] {
            return Some(len - 1 - i);
        }
    }
    None
}

fn split_all_paths(paths: Vec<PathSegment>) -> Vec<PathSegment> {
    let mut paths = paths.clone();

    let mut has_cross = true;
    let mut i_min = 0;
    while has_cross {
        'outer: {
            for i in i_min..paths.len() {
                for j in i + 1..paths.len() {
                    if let Some((mut a, mut b)) = split_line_on_cross_point(&paths[i], &paths[j]) {
                        println!("i: {:?}, j: {:?}", i, j);
                        println!("path_i: {:?}, path_j: {:?}", &paths[i], &paths[j]);
                        println!("a: {:?}, b: {:?}", a, b);

                        has_cross = true;
                        let mut result = Vec::new();

                        result.append(&mut paths.clone()[0..i].to_vec());

                        result.append(&mut a);
                        result.append(&mut b);
                        if i + 1 != j {
                            result.append(&mut paths.clone()[i + 1..j].to_vec());
                        }
                        result.append(&mut paths.clone()[j + 1..].to_vec());
                        paths = result;
                        break 'outer;
                    }
                }
                i_min = i;
            }
            has_cross = false;
        }
    }
    paths
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
    if (0.0..1.0).contains(&ua) && (0.0..1.0).contains(&ub) {
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
                    // 交点が線分の端点に近い場合は端点として扱う
                    fn normalize(value: f32) -> f32 {
                        const NORMALIZE_EPSILON: f32 = 0.01;
                        if 0.0 < value && value < NORMALIZE_EPSILON {
                            0.0
                        } else if 1.0 - NORMALIZE_EPSILON < value && value < 1.0 {
                            1.0
                        } else {
                            value
                        }
                    }

                    let cp = CrossPoint {
                        point: point.point,
                        a_position: normalize(a_position + point.a_position * gain),
                        b_position: normalize(b_position + point.b_position * gain),
                    };

                    if !points.contains(&cp) {
                        points.push(CrossPoint {
                            point: point.point,
                            a_position: normalize(a_position + point.a_position * gain),
                            b_position: normalize(b_position + point.b_position * gain),
                        })
                    }
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

    use std::{cmp::Ordering, f32::consts::PI, fs::File};

    use rustybuzz::{Face, ttf_parser::OutlineBuilder};
    use tiny_skia::Path;
    use tiny_skia_path::{NormalizedF32Exclusive, Point, path_geometry};

    use crate::{
        Cubic, EPSILON, Line, PathSegment, Quadratic, cross_point, cross_point_line,
        get_loop_segment, has_vector_tail_loop, is_clockwise, is_closed, path_to_path_segments,
        paths_to_path_segments, remove_overlap, remove_overlap_rev, reverse, same_path,
        split_all_paths, split_line_on_cross_point,
        test_helper::{TestPathBuilder, path_segments_to_image, path_segments_to_images},
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

    #[test]
    fn test_font() {
        let font_file = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");
        let face: Face = Face::from_slice(font_file, 0).unwrap();
        let glyph_id = face.glyph_index('🐢').unwrap();
        let mut path_builder = TestPathBuilder::new();
        face.outline_glyph(glyph_id, &mut path_builder).unwrap();
        let paths = path_builder.paths();

        let segments: Vec<PathSegment> = paths
            .iter()
            .flat_map(|path| path_to_path_segments(path.clone()))
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
    fn test_turtle() {
        let font_file = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");
        let face: Face = Face::from_slice(font_file, 0).unwrap();
        let glyph_id = face.glyph_index('🐢').unwrap();
        let mut path_builder = TestPathBuilder::new();
        face.outline_glyph(glyph_id, &mut path_builder).unwrap();
        let paths = path_builder.paths();

        visualize_paths(paths);
    }

    #[test]
    fn test_pig() {
        let font_file = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");
        let face: Face = Face::from_slice(font_file, 0).unwrap();
        let glyph_id = face.glyph_index('🐖').unwrap();
        let mut path_builder = TestPathBuilder::new();
        face.outline_glyph(glyph_id, &mut path_builder).unwrap();
        let paths = path_builder.paths();

        visualize_paths(paths);
    }

    #[test]
    fn test_duck() {
        let font_file = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");
        let face: Face = Face::from_slice(font_file, 0).unwrap();
        let glyph_id = face.glyph_index('🐦').unwrap();
        let mut path_builder = TestPathBuilder::new();
        face.outline_glyph(glyph_id, &mut path_builder).unwrap();
        let paths = path_builder.paths();

        visualize_paths(paths);
    }

    #[test]
    fn test_kadomatsu() {
        let font_file = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");
        let face: Face = Face::from_slice(font_file, 0).unwrap();
        let glyph_id = face.glyph_index('🎍').unwrap();
        let mut path_builder = TestPathBuilder::new();
        face.outline_glyph(glyph_id, &mut path_builder).unwrap();
        let paths = path_builder.paths();

        visualize_paths(paths);
    }

    #[test]
    fn test_hinode() {
        let font_file = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");
        let face: Face = Face::from_slice(font_file, 0).unwrap();
        let glyph_id = face.glyph_index('🌅').unwrap();
        let mut path_builder = TestPathBuilder::new();
        face.outline_glyph(glyph_id, &mut path_builder).unwrap();
        let paths = path_builder.paths();

        visualize_paths(paths);
    }

    #[test]
    fn test_man() {
        let mut path_builder = TestPathBuilder::new();
        path_builder.move_to(1.0, 0.0);
        path_builder.line_to(2.0, 0.0);
        path_builder.line_to(2.0, 3.0);
        path_builder.line_to(1.0, 3.0);
        path_builder.line_to(1.0, 0.0);
        path_builder.close();
        path_builder.move_to(0.0, 1.0);
        path_builder.line_to(0.0, 2.0);
        path_builder.line_to(3.0, 2.0);
        path_builder.line_to(3.0, 1.0);
        path_builder.line_to(0.0, 1.0);
        path_builder.close();

        let paths = path_builder.paths();

        visualize_paths(paths);
    }

    fn visualize_paths(paths: Vec<Path>) {
        let image_dir_path = std::path::Path::new("image");
        if !image_dir_path.exists() {
            std::fs::create_dir(image_dir_path).unwrap();
        }
        image_dir_path.read_dir().unwrap().for_each(|entry| {
            if let Ok(entry) = entry {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| if ext == "png" { Some(entry) } else { None })
                    .map(|f| {
                        let _ = std::fs::remove_file(f.path());
                    });
            }
        });

        let segments = paths_to_path_segments(&paths);
        let segments = split_all_paths(segments);

        let no_zero_segment = segments.iter().all(|seg| {
            let (f, t) = seg.endpoints();
            f != t
        });

        {
            // オリジナル
            path_segments_to_images("origin", segments.iter().collect(), vec![]);
        }
        {
            // 時計回りでループを取得
            let clockwise = get_loop_segment(segments.clone(), true);
            clockwise.into_iter().enumerate().for_each(|(i, segments)| {
                path_segments_to_images(
                    &format!("clockwise_{}_{}", i, is_clockwise(&segments)),
                    segments.iter().collect(),
                    vec![],
                );
            });

            // 反時計回りでループを取得
            let counter_clockwise = get_loop_segment(segments.clone(), false);
            counter_clockwise
                .into_iter()
                .enumerate()
                .for_each(|(i, segments)| {
                    path_segments_to_images(
                        &format!("counter_clockwise_{}_{}", i, is_clockwise(&segments)),
                        segments.iter().collect(),
                        vec![],
                    );
                });
        }

        {
            let segments = remove_overlap(paths.clone());
            path_segments_to_images("generated", segments.iter().flatten().collect(), vec![]);
            segments.into_iter().enumerate().for_each(|(i, segments)| {
                println!(
                    "num:{}, clockwise:{}, is_clsed:{}, len:{}",
                    i,
                    is_clockwise(&segments),
                    is_closed(&segments),
                    segments.len()
                );
                path_segments_to_images(
                    &format!("remove_overlap_{}_{}", i, is_clockwise(&segments)),
                    segments.iter().collect(),
                    vec![],
                );
            });
        }
        println!("no_zero_segment: {}", no_zero_segment);
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
    fn test_has_vector_tail_loop() {
        // 例) vec![1,2,3,4,5] の場合ループが発生していないため None を返す。
        {
            let sut = vec![1, 2, 3, 4, 5];
            let result = has_vector_tail_loop(&sut);
            assert_eq!(result, None);
        }
        // 例) vec![1,2,3,4,5,6,4,5,6] の場合、末尾からみてループの開始場所のインデックス Some(6) を返す。
        {
            let sut = vec![1, 2, 3, 4, 5, 6, 4, 5, 6];
            let result = has_vector_tail_loop(&sut);
            assert_eq!(result, Some(6));
        }
        {
            let sut = vec!['h', 'o', 'g', 'e', 'o', 'g', 'e'];
            let result = has_vector_tail_loop(&sut);
            assert_eq!(result, Some(4));
        }
    }

    #[test]
    fn test_list_cross() {
        let span = 20;
        {
            let p1: Point = (1.0, 0.0).into();
            for i in 0..span {
                let step = 2.0 * PI * i as f32 / span as f32;
                let mut p2: Point = (step.cos(), step.sin()).into();
                println!(
                    "外積: {:+.3},\t内積: {:+.3}, {:?}",
                    p1.cross(p2),
                    p1.dot(p2),
                    p2
                );
            }
        }
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
    }
}
