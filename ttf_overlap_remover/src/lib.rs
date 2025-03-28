use std::collections::HashMap;

use cross_point::split_line_on_cross_point;
pub use outline_builder::OverlapRemoveOutlineBuilder;
use path_segment::{Cubic, Line, PathSegment, Quadratic, SegmentTrait};
use tiny_skia_path::{Path, PathBuilder, Point};
use util::cmp_clockwise;

mod cross_point;
mod outline_builder;
mod path_segment;
#[cfg(test)]
mod test_helper;
mod util;

/// Point を PathSegment に変換する
#[allow(dead_code)]
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
#[allow(dead_code)]
#[inline]
fn paths_to_path_segments(paths: &[Path]) -> Vec<PathSegment> {
    paths
        .iter()
        .flat_map(|path| path_to_path_segments(path.clone()))
        .collect()
}

/// Vec<Path> を PathSegment に変換する
#[allow(dead_code)]
#[inline]
fn paths_to_clockwise_path_segments(paths: &[Path]) -> (Vec<PathSegment>, Vec<PathSegment>) {
    let path_segments: Vec<Vec<PathSegment>> = paths
        .iter()
        .map(|path| path_to_path_segments(path.clone()))
        .collect();
    let clock_wise = path_segments
        .iter()
        .filter(|segments| is_clockwise(segments))
        .flat_map(|segments| segments.clone())
        .collect();
    let counter_clock_wise = path_segments
        .iter()
        .filter(|segments| !is_clockwise(segments))
        .flat_map(|segments| segments.clone())
        .collect();
    (clock_wise, counter_clock_wise)
}

#[derive(Debug, Clone)]
struct LoopSegment {
    segments: Vec<PathSegment>,
}

impl LoopSegment {
    fn create(segments: Vec<PathSegment>) -> Option<Self> {
        let result = Self { segments };
        if !result.is_closed() {
            return None;
        }
        Some(result)
    }

    fn is_clockwise(&self) -> bool {
        is_clockwise(&self.segments)
    }

    fn is_closed(&self) -> bool {
        is_closed(&self.segments)
    }

    fn reverse(&self) -> Self {
        Self {
            segments: self.segments.iter().map(|s| s.reverse()).collect(),
        }
    }

    fn same_path(&self, other: &Self) -> bool {
        same_path(&self.segments, &other.segments)
    }

    fn to_path(&self) -> Option<Path> {
        let mut pb = PathBuilder::new();
        let mut first_segment = true;
        for segment in self.segments.iter() {
            if first_segment {
                let Point { x, y } = segment.endpoints().0;
                pb.move_to(x, y);
                first_segment = false;
            }
            match segment {
                PathSegment::Line(line) => {
                    pb.line_to(line.to.x, line.to.y);
                }
                PathSegment::Quadratic(quad) => {
                    pb.quad_to(quad.control.x, quad.control.y, quad.to.x, quad.to.y);
                }
                PathSegment::Cubic(cubic) => {
                    pb.cubic_to(
                        cubic.control1.x,
                        cubic.control1.y,
                        cubic.control2.x,
                        cubic.control2.y,
                        cubic.to.x,
                        cubic.to.y,
                    );
                }
            }
        }
        pb.close();
        pb.finish()
    }
}

#[inline]
fn is_closed(segments: &[PathSegment]) -> bool {
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

#[inline]
fn is_clockwise(segments: &Vec<PathSegment>) -> bool {
    let mut sum = 0.0;
    for segment in segments {
        let (from, to) = segment.endpoints();
        //sum += from.cross(to);
        sum += from.x * to.y - from.y * to.x;
    }
    sum > 0.0
}

#[allow(dead_code)]
fn reverse(segments: &[PathSegment]) -> Vec<PathSegment> {
    segments.iter().map(|s| s.reverse()).rev().collect()
}

fn same_path(segments1: &[PathSegment], segments2: &[PathSegment]) -> bool {
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

pub fn remove_path_overlap(paths: Vec<Path>) -> Vec<Path> {
    remove_overlap(paths)
        .iter()
        .flat_map(|segments| segments.to_path())
        .collect()
}

#[allow(dead_code)]
pub(crate) fn remove_overlap(paths: Vec<Path>) -> Vec<LoopSegment> {
    // Path を全て PathFlagment に分解し、交差部分でセグメントを分割する
    let path_segments = paths
        .iter()
        .flat_map(|path| path_to_path_segments(path.clone()));
    let path_segments = split_all_paths(path_segments.collect());
    remove_overlap_inner(path_segments)
}

fn get_loop_segment(path_segments: Vec<PathSegment>, clock_wise: bool) -> Vec<LoopSegment> {
    // 分解された PathFlagment からつなげてパスの候補となる Vec<PathSegment> を構成する
    let mut result_paths: Vec<LoopSegment> = Vec::new();

    for segment in path_segments.iter() {
        let mut current_segment = segment.clone();
        let mut current_path = Vec::new();
        current_path.push(current_segment.clone());
        loop {
            // 次のパスになりうるセグメントを探す(current の to が next の from または to と一致するセグメント)
            let nexts: Vec<PathSegment> = path_segments
                .iter()
                //.flat_map(|p| [p.clone(), p.reverse()])
                .flat_map(|p| [p.clone()])
                // 今のセグメントと繋がるパスを絞り込む
                .filter(|s| {
                    let (_, current_to) = current_segment.endpoints();
                    let (next_from, _) = s.endpoints();
                    current_to == next_from
                })
                // 今のセグメントと同一または逆向きのセグメントは除外
                .filter(|s| !s.is_same_or_reversed(&current_segment))
                .collect();
            if nexts.is_empty() {
                // 次のパスになりうるセグメントが見つからない場合、閉じていない Path だった可能性もあるのでまぁいいかという感じで次のセグメントに進む
                break;
            }
            // 現在のセグメントの進行方向から、最も左向きのベクトルを持つセグメントを次のセグメントとして選択する
            current_segment = if clock_wise {
                current_segment.select_clockwise_vector(&nexts)
            } else {
                current_segment.select_counter_clockwise_vector(&nexts)
            };
            current_path.push(current_segment.clone());

            // ループが発生している場合、ループを切り出して result_paths に追加する
            if let Some(loop_position) = has_vector_tail_loop(&current_path) {
                let created_path =
                    LoopSegment::create(current_path.split_off(loop_position)).unwrap();
                let has_same_path = result_paths
                    .iter()
                    .any(|s| s.same_path(&created_path) || s.same_path(&created_path.reverse()));
                if !has_same_path {
                    result_paths.push(created_path);
                }
                break;
            }
        }
    }
    result_paths
}

fn get_splitted_loop_segment(path_segments: Vec<PathSegment>, clock_wise: bool) -> LoopSegments {
    let result_paths = get_loop_segment(path_segments.clone(), clock_wise);

    let clockwise: Vec<LoopSegment> = result_paths
        .iter()
        .filter(|segments| segments.is_clockwise())
        .cloned()
        .collect();
    let counter_clockwise: Vec<LoopSegment> = result_paths
        .iter()
        .filter(|segments| !segments.is_clockwise())
        .cloned()
        .collect();

    LoopSegments {
        clockwise,
        counter_clockwise,
    }
}

struct LoopSegments {
    clockwise: Vec<LoopSegment>,
    counter_clockwise: Vec<LoopSegment>,
}

/// overlap が含まれる path を受け取り、overlap を除去した path を返す
fn remove_overlap_inner(path_segments: Vec<PathSegment>) -> Vec<LoopSegment> {
    // 分解された PathFlagment からつなげてパスの候補となる Vec<PathSegment> を構成する
    let loop_segments = get_splitted_loop_segment(path_segments.clone(), false);
    //let mut result = loop_segments.clockwise.clone();
    //result.append(&mut loop_segments.filterd_clockwise());
    let mut result = loop_segments.clockwise.clone();
    result.append(&mut loop_segments.counter_clockwise.clone());
    result
}

/// 末尾にループが発生している時にループの開始位置を返す関数。
fn has_vector_tail_loop<T: PartialEq>(value: &[T]) -> Option<usize> {
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
            let i_start = i_min;
            for i in i_start..paths.len() {
                for j in i + 1..paths.len() {
                    if let Some((mut a, mut b)) = split_line_on_cross_point(&paths[i], &paths[j]) {
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

#[cfg(test)]
mod tests {

    use std::f32::consts::PI;

    use rustybuzz::{Face, ttf_parser::OutlineBuilder};
    use tiny_skia::Path;
    use tiny_skia_path::Point;

    use crate::{
        OverlapRemoveOutlineBuilder, get_loop_segment, has_vector_tail_loop,
        paths_to_path_segments, remove_overlap, split_all_paths,
        test_helper::path_segments_to_images,
    };

    #[test]
    fn test_turtle() {
        let font_file = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");
        let face: Face = Face::from_slice(font_file, 0).unwrap();
        let glyph_id = face.glyph_index('🐢').unwrap();
        let mut path_builder = OverlapRemoveOutlineBuilder::new();
        face.outline_glyph(glyph_id, &mut path_builder).unwrap();
        let paths = path_builder.paths();

        visualize_paths(paths);
    }

    #[test]
    fn test_pig() {
        let font_file = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");
        let face: Face = Face::from_slice(font_file, 0).unwrap();
        let glyph_id = face.glyph_index('🐖').unwrap();
        let mut path_builder = OverlapRemoveOutlineBuilder::new();
        face.outline_glyph(glyph_id, &mut path_builder).unwrap();
        let paths = path_builder.paths();

        visualize_paths(paths);
    }

    #[test]
    fn test_duck() {
        let font_file = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");
        let face: Face = Face::from_slice(font_file, 0).unwrap();
        let glyph_id = face.glyph_index('🐦').unwrap();
        let mut path_builder = OverlapRemoveOutlineBuilder::new();
        face.outline_glyph(glyph_id, &mut path_builder).unwrap();
        let paths = path_builder.paths();

        visualize_paths(paths);
    }

    #[test]
    fn test_kadomatsu() {
        let font_file = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");
        let face: Face = Face::from_slice(font_file, 0).unwrap();
        let glyph_id = face.glyph_index('🎍').unwrap();
        let mut path_builder = OverlapRemoveOutlineBuilder::new();
        face.outline_glyph(glyph_id, &mut path_builder).unwrap();
        let paths = path_builder.paths();

        visualize_paths(paths);
    }

    #[test]
    fn test_hinode() {
        let font_file = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");
        let face: Face = Face::from_slice(font_file, 0).unwrap();
        let glyph_id = face.glyph_index('🌅').unwrap();
        let mut path_builder = OverlapRemoveOutlineBuilder::new();
        face.outline_glyph(glyph_id, &mut path_builder).unwrap();
        let paths = path_builder.paths();

        visualize_paths(paths);
    }

    #[test]
    fn test_dog() {
        let font_file = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");
        let face: Face = Face::from_slice(font_file, 0).unwrap();
        let glyph_id = face.glyph_index('🐕').unwrap();
        let mut path_builder = OverlapRemoveOutlineBuilder::new();
        face.outline_glyph(glyph_id, &mut path_builder).unwrap();
        let paths = path_builder.paths();

        visualize_paths(paths);
    }

    #[test]
    fn test_man() {
        let mut path_builder = OverlapRemoveOutlineBuilder::new();
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
                if let Some(f) = entry
                    .path()
                    .extension()
                    .and_then(|ext| if ext == "png" { Some(entry) } else { None })
                {
                    let _ = std::fs::remove_file(f.path());
                }
            }
        });

        let segments = paths_to_path_segments(&paths);
        let segments = split_all_paths(segments);

        let no_zero_segment = segments.iter().all(|seg| {
            let (f, t) = seg.endpoints();
            f != t
        });

        let min_distance = segments.iter().min_by(|l, r| {
            let l_dis = {
                let (f, t) = l.endpoints();
                f.distance(t)
            };
            let r_dis = {
                let (f, t) = r.endpoints();
                f.distance(t)
            };
            l_dis.partial_cmp(&r_dis).unwrap()
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
                    &format!("clockwise_{}_{}", i, segments.is_clockwise()),
                    segments.segments.iter().collect(),
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
                        &format!("counter_clockwise_{}_{}", i, segments.is_clockwise()),
                        segments.segments.iter().collect(),
                        vec![],
                    );
                });
        }

        {
            let segments = remove_overlap(paths.clone());
            path_segments_to_images(
                "generated",
                segments.iter().flat_map(|s| &s.segments).collect(),
                vec![],
            );
            segments.into_iter().enumerate().for_each(|(i, segments)| {
                println!(
                    "num:{}, clockwise:{}, is_clsed:{}, len:{}",
                    i,
                    segments.is_clockwise(),
                    segments.is_closed(),
                    segments.segments.len()
                );
                path_segments_to_images(
                    &format!("remove_overlap_{}_{}", i, segments.is_clockwise()),
                    segments.segments.iter().collect(),
                    vec![],
                );
            });
        }
        println!("no_zero_segment: {}", no_zero_segment);
        println!("min_distance: {:?}", min_distance);
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
                let p2: Point = (step.cos(), step.sin()).into();
                println!(
                    "外積: {:+.3},\t内積: {:+.3}, {:?}",
                    p1.cross(p2),
                    p1.dot(p2),
                    p2
                );
            }
        }
    }
}
