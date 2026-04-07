//! ttf_overlap_remover2 — TTF フォントグリフのオーバーラップ除去
//!
//! 複数のサブパスで構成されるグリフアウトラインから重複領域を除去する。
//!
//! ## アルゴリズム概要
//!
//! 1. 各サブパスをベジェセグメント列に分解
//! 2. 全セグメントペア間で交差点を検出（Newton-Raphson 法 + 二分探索のハイブリッド）
//! 3. 交差点でセグメントを分割し、グラフ構造を構築
//! 4. ワインディングナンバーを用いて各エッジの内外を判定
//! 5. 外側境界のエッジのみ残してパスを再構成

pub use outline_builder::OverlapRemoveOutlineBuilder;

mod cross_point;
mod flatpath;
mod outline_builder;
mod path_segment;
mod winding;

use path_segment::PathSegment;
use tiny_skia_path::{Path, PathBuilder, Point};

/// Path を PathSegment に変換する
#[allow(dead_code)]
fn path_to_path_segments(path: &Path) -> Vec<PathSegment> {
    let mut results = Vec::new();
    let mut start_point = Point::default();
    for segment in path.segments() {
        match segment {
            tiny_skia_path::PathSegment::MoveTo(point) => start_point = point,
            tiny_skia_path::PathSegment::LineTo(point) => {
                if start_point != point {
                    results.push(PathSegment::Line(path_segment::Line {
                        from: start_point,
                        to: point,
                    }));
                }
                start_point = point;
            }
            tiny_skia_path::PathSegment::QuadTo(control, point) => {
                if start_point != point {
                    results.push(PathSegment::Quadratic(path_segment::Quadratic {
                        from: start_point,
                        to: point,
                        control,
                    }));
                }
                start_point = point;
            }
            tiny_skia_path::PathSegment::CubicTo(control1, control2, point) => {
                if start_point != point {
                    results.push(PathSegment::Cubic(path_segment::Cubic {
                        from: start_point,
                        to: point,
                        control1,
                        control2,
                    }));
                }
                start_point = point;
            }
            tiny_skia_path::PathSegment::Close => {}
        }
    }
    results
}

/// Path をサブパス（Close で区切られた一連のセグメント）に分解する
fn path_to_subpaths(path: &Path) -> Vec<Vec<PathSegment>> {
    let mut subpaths = Vec::new();
    let mut current = Vec::new();
    let mut start_point = Point::default();

    for segment in path.segments() {
        match segment {
            tiny_skia_path::PathSegment::MoveTo(point) => {
                if !current.is_empty() {
                    subpaths.push(current);
                    current = Vec::new();
                }
                start_point = point;
            }
            tiny_skia_path::PathSegment::LineTo(point) => {
                if start_point != point {
                    current.push(PathSegment::Line(path_segment::Line {
                        from: start_point,
                        to: point,
                    }));
                }
                start_point = point;
            }
            tiny_skia_path::PathSegment::QuadTo(control, point) => {
                if start_point != point {
                    current.push(PathSegment::Quadratic(path_segment::Quadratic {
                        from: start_point,
                        to: point,
                        control,
                    }));
                }
                start_point = point;
            }
            tiny_skia_path::PathSegment::CubicTo(control1, control2, point) => {
                if start_point != point {
                    current.push(PathSegment::Cubic(path_segment::Cubic {
                        from: start_point,
                        to: point,
                        control1,
                        control2,
                    }));
                }
                start_point = point;
            }
            tiny_skia_path::PathSegment::Close => {
                // Close 時に暗黙の LineTo（現在位置 → MoveTo 位置）を生成
                if !current.is_empty() {
                    let (_, last_to) = current.last().unwrap().endpoints();
                    let (first_from, _) = current.first().unwrap().endpoints();
                    if last_to != first_from {
                        current.push(PathSegment::Line(path_segment::Line {
                            from: last_to,
                            to: first_from,
                        }));
                    }
                    subpaths.push(current);
                    current = Vec::new();
                }
            }
        }
    }
    if !current.is_empty() {
        subpaths.push(current);
    }
    subpaths
}

/// セグメント列からサブパスの向き（時計回りか）を判定する
/// Shoelace formula で符号付き面積を計算
fn is_clockwise(segments: &[PathSegment]) -> bool {
    let mut sum = 0.0f64;
    for seg in segments {
        // ベジェ曲線をフラット化して面積を近似する
        let points = seg.flatten(0.25);
        for pair in points.windows(2) {
            sum +=
                (pair[0].x as f64) * (pair[1].y as f64) - (pair[1].x as f64) * (pair[0].y as f64);
        }
    }
    // Y 軸が上向き（フォント座標系）で sum > 0 が時計回り
    sum > 0.0
}

/// 全サブパスの和集合を求めてオーバーラップを除去する
pub(crate) fn remove_path_overlap(paths: Vec<Path>) -> Vec<Path> {
    // 全パスからサブパスを収集
    let mut all_subpaths: Vec<Vec<PathSegment>> = Vec::new();
    for path in &paths {
        let subpaths = path_to_subpaths(path);
        all_subpaths.extend(subpaths);
    }

    if all_subpaths.is_empty() {
        return vec![];
    }

    // 全セグメントを集める
    let mut all_segments: Vec<PathSegment> = all_subpaths.iter().flatten().cloned().collect();

    // 交差点で全セグメントを分割する
    all_segments = split_all_segments(all_segments);

    // 逆向きペアを打ち消す
    all_segments = cancel_reversed_segments(all_segments);

    // セグメントからループを再構成
    let loops = build_loops(&all_segments);

    // 各ループについて、外側境界に属するかを winding number で判定
    let result_loops = filter_loops_by_winding(&loops, &all_subpaths);

    // Path に変換
    result_loops
        .iter()
        .filter_map(|loop_seg| segments_to_path(loop_seg))
        .collect()
}

/// セグメント列を Path に変換
fn segments_to_path(segments: &[PathSegment]) -> Option<Path> {
    if segments.is_empty() {
        return None;
    }
    let mut pb = PathBuilder::new();
    for (i, seg) in segments.iter().enumerate() {
        if i == 0 {
            let (from, _) = seg.endpoints();
            pb.move_to(from.x, from.y);
        }
        match seg {
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

/// 全セグメントペア間の交差点を見つけて分割する
fn split_all_segments(mut segments: Vec<PathSegment>) -> Vec<PathSegment> {
    // 交差点検出と分割を反復的に行う
    let mut changed = true;
    while changed {
        changed = false;
        let mut i = 0;
        while i < segments.len() {
            let mut j = i + 1;
            while j < segments.len() {
                if segments[i].is_same_or_reversed(&segments[j]) {
                    j += 1;
                    continue;
                }

                let cross_points = cross_point::find_cross_points(&segments[i], &segments[j]);
                if cross_points.is_empty() {
                    j += 1;
                    continue;
                }

                // 分割する
                let seg_i = segments.remove(i);
                // j のインデックスを調整
                let j_adj = if j > i { j - 1 } else { j };
                let seg_j = segments.remove(j_adj);

                let split_i = split_segment_at_points(&seg_i, &cross_points, true);
                let split_j = split_segment_at_points(&seg_j, &cross_points, false);

                // 分割結果を挿入
                let insert_pos = i.min(j_adj);
                for (k, s) in split_i.into_iter().chain(split_j.into_iter()).enumerate() {
                    segments.insert(insert_pos + k, s);
                }

                changed = true;
                break;
            }
            if changed {
                break;
            }
            i += 1;
        }
    }
    segments
}

/// 交差点でセグメントを分割する
fn split_segment_at_points(
    segment: &PathSegment,
    cross_points: &[cross_point::CrossPoint],
    is_a: bool,
) -> Vec<PathSegment> {
    let mut positions: Vec<f32> = cross_points
        .iter()
        .map(|cp| if is_a { cp.t_a } else { cp.t_b })
        .filter(|&t| t > EPSILON && t < 1.0 - EPSILON)
        .collect();
    positions.sort_by(|a, b| a.partial_cmp(b).unwrap());
    positions.dedup_by(|a, b| (*a - *b).abs() < EPSILON);

    if positions.is_empty() {
        return vec![segment.clone()];
    }

    let mut result = Vec::new();
    let mut remaining = segment.clone();
    let mut consumed = 0.0f32;

    for &t in &positions {
        let adjusted_t = (t - consumed) / (1.0 - consumed);
        if adjusted_t <= EPSILON || adjusted_t >= 1.0 - EPSILON {
            continue;
        }
        let (pre, post) = remaining.chop(adjusted_t);

        // 交差点の座標を取得して端点を合わせる
        let cross_point = segment.evaluate(t);
        let mut pre = pre;
        let mut post = post;
        pre.set_to(cross_point);
        post.set_from(cross_point);

        if !pre.is_degenerate() {
            result.push(pre);
        }
        remaining = post;
        consumed = t;
    }
    if !remaining.is_degenerate() {
        result.push(remaining);
    }
    result
}

const EPSILON: f32 = 1e-5;
const CANCEL_EPSILON: f32 = 0.05;

/// 逆向きのセグメントペアを打ち消す
fn cancel_reversed_segments(segments: Vec<PathSegment>) -> Vec<PathSegment> {
    let mut removed = vec![false; segments.len()];

    for i in 0..segments.len() {
        if removed[i] {
            continue;
        }
        for j in (i + 1)..segments.len() {
            if removed[j] {
                continue;
            }
            if segments[i].is_approximately_reversed(&segments[j], CANCEL_EPSILON) {
                removed[i] = true;
                removed[j] = true;
                break;
            }
        }
    }

    segments
        .into_iter()
        .enumerate()
        .filter_map(|(i, s)| if removed[i] { None } else { Some(s) })
        .collect()
}

/// セグメントからループ（閉じたパス）を構築する
fn build_loops(segments: &[PathSegment]) -> Vec<Vec<PathSegment>> {
    use std::collections::HashMap;

    if segments.is_empty() {
        return vec![];
    }

    let mut used = vec![false; segments.len()];
    let mut loops = Vec::new();

    // エンドポイントのインデックスマップを構築
    // from ポイントでグループ化
    let mut from_map: HashMap<PointKey, Vec<usize>> = HashMap::new();
    for (i, seg) in segments.iter().enumerate() {
        let (from, _) = seg.endpoints();
        let key = PointKey::new(from);
        from_map.entry(key).or_default().push(i);
    }

    for start_idx in 0..segments.len() {
        if used[start_idx] {
            continue;
        }

        let mut path = Vec::new();
        let mut current_idx = start_idx;
        let mut success = false;

        loop {
            if used[current_idx] {
                // 既に使用済みならループ構築失敗
                break;
            }
            used[current_idx] = true;
            path.push(current_idx);

            let (_, to) = segments[current_idx].endpoints();
            let to_key = PointKey::new(to);

            // 開始点に戻ったらループ完成
            let (start_from, _) = segments[start_idx].endpoints();
            if path.len() > 1 && to.distance(start_from) < EPSILON {
                success = true;
                break;
            }

            // 次のセグメントを探す
            let Some(candidates) = from_map.get(&to_key) else {
                break;
            };

            // 次のセグメント: 現在のセグメントの進行方向から最も時計回りに近いものを選ぶ
            let current_seg = &segments[current_idx];
            let mut best_idx = None;
            let mut best_angle = f64::MAX;

            let base_vec = current_seg.to_vector();
            let base_angle = (-(base_vec.y as f64)).atan2(-(base_vec.x as f64));

            for &cand_idx in candidates {
                if used[cand_idx] {
                    continue;
                }
                if segments[cand_idx].is_same_or_reversed(current_seg) {
                    continue;
                }
                let cand_vec = segments[cand_idx].from_vector();
                let cand_angle = (cand_vec.y as f64).atan2(cand_vec.x as f64);

                // base_angle (反転した進入ベクトル) からの時計回り角度差
                let mut diff = cand_angle - base_angle;
                if diff <= 0.0 {
                    diff += 2.0 * std::f64::consts::PI;
                }
                if diff < best_angle {
                    best_angle = diff;
                    best_idx = Some(cand_idx);
                }
            }

            match best_idx {
                Some(idx) => current_idx = idx,
                None => break,
            }
        }

        if success {
            let loop_segments: Vec<PathSegment> =
                path.iter().map(|&i| segments[i].clone()).collect();
            loops.push(loop_segments);
        } else {
            // ループ構築に失敗した場合、使用フラグを戻す
            for &idx in &path {
                used[idx] = false;
            }
        }
    }

    // 使用されなかったセグメントについて、よりゆるい条件で再試行
    let unused_segments: Vec<PathSegment> = segments
        .iter()
        .enumerate()
        .filter(|&(i, _)| !used[i])
        .map(|(_, s)| s.clone())
        .collect();

    if !unused_segments.is_empty() {
        let extra_loops = build_loops_greedy(&unused_segments);
        loops.extend(extra_loops);
    }

    loops
}

/// よりゆるい条件でループを構築する（距離ベースの近傍マッチング）
fn build_loops_greedy(segments: &[PathSegment]) -> Vec<Vec<PathSegment>> {
    let mut used = vec![false; segments.len()];
    let mut loops = Vec::new();

    for start_idx in 0..segments.len() {
        if used[start_idx] {
            continue;
        }

        let mut path = vec![start_idx];
        let mut visited = vec![false; segments.len()];
        visited[start_idx] = true;

        loop {
            let current_idx = *path.last().unwrap();
            let (_, to) = segments[current_idx].endpoints();

            // 開始点に戻れるか
            let (start_from, _) = segments[start_idx].endpoints();
            if path.len() > 1 && to.distance(start_from) < CANCEL_EPSILON {
                // ループ完成
                for &idx in &path {
                    used[idx] = true;
                }
                let loop_segments: Vec<PathSegment> =
                    path.iter().map(|&i| segments[i].clone()).collect();
                loops.push(loop_segments);
                break;
            }

            // 次のセグメントを距離ベースで探す
            let mut best_idx = None;
            let mut best_dist = CANCEL_EPSILON;
            for (i, seg) in segments.iter().enumerate() {
                if visited[i] || used[i] {
                    continue;
                }
                let (from, _) = seg.endpoints();
                let dist = to.distance(from);
                if dist < best_dist {
                    best_dist = dist;
                    best_idx = Some(i);
                }
            }

            match best_idx {
                Some(idx) => {
                    visited[idx] = true;
                    path.push(idx);
                }
                None => break,
            }
        }
    }

    loops
}

/// ワインディングナンバーに基づいてループをフィルタリング
///
/// 各ループの「辺を少し法線方向にずらした内部点」でのワインディングナンバーを計算し、
/// non-zero fill rule に基づいてフィルタリングする。
fn filter_loops_by_winding(
    loops: &[Vec<PathSegment>],
    original_subpaths: &[Vec<PathSegment>],
) -> Vec<Vec<PathSegment>> {
    if loops.is_empty() {
        return vec![];
    }

    let mut result = Vec::new();

    for loop_seg in loops {
        // ループの辺に垂直な内側方向に少しずらした点を取得
        let test_point = loop_interior_sample_point(loop_seg);

        let mut total_winding = 0i32;
        for subpath in original_subpaths {
            total_winding += winding::winding_number(test_point, subpath);
        }

        // non-zero rule: winding != 0 なら塗りつぶし領域に属するループ
        if total_winding != 0 {
            result.push(loop_seg.clone());
        }
    }

    result
}

/// ループの内部にあるサンプル点を推定する
/// セグメントの中点から法線方向に微小量ずらした点を返す
fn loop_interior_sample_point(segments: &[PathSegment]) -> Point {
    if segments.is_empty() {
        return Point::zero();
    }

    // ループの面積の符号から左右を判定
    let cw = is_clockwise(segments);

    // 最初のセグメントの中点での法線を使う
    let seg = &segments[0];
    let mid = seg.evaluate(0.5);
    let tangent = {
        let p0 = seg.evaluate(0.49);
        let p1 = seg.evaluate(0.51);
        let dx = p1.x - p0.x;
        let dy = p1.y - p0.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-10 {
            Point::from_xy(1.0, 0.0)
        } else {
            Point::from_xy(dx / len, dy / len)
        }
    };

    // 法線方向（時計回りかどうかで向きを決定）
    let offset = 0.1; // 微小オフセット
    let normal = if cw {
        // CW ループ（フォント座標系 Y上向き）の場合、左側が内側
        Point::from_xy(-tangent.y, tangent.x)
    } else {
        // CCW ループの場合、右側が内側
        Point::from_xy(tangent.y, -tangent.x)
    };

    Point::from_xy(mid.x + normal.x * offset, mid.y + normal.y * offset)
}

/// 座標のハッシュキー（浮動小数点の近傍をまとめる）
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct PointKey {
    x: i32,
    y: i32,
}

impl PointKey {
    fn new(p: Point) -> Self {
        // 0.01 単位で量子化
        Self {
            x: (p.x * 100.0).round() as i32,
            y: (p.y * 100.0).round() as i32,
        }
    }
}

#[cfg(test)]
mod tests {
    use rustybuzz::{Face, ttf_parser::OutlineBuilder};
    use tiny_skia_path::Point;

    use crate::{
        OverlapRemoveOutlineBuilder, is_clockwise,
        path_segment::{Line, PathSegment},
    };

    #[test]
    fn test_cross_shape() {
        // 十字型の2つの長方形の重なりをテスト
        let mut builder = OverlapRemoveOutlineBuilder::default();
        // 縦の長方形
        builder.move_to(1.0, 0.0);
        builder.line_to(2.0, 0.0);
        builder.line_to(2.0, 3.0);
        builder.line_to(1.0, 3.0);
        builder.close();
        // 横の長方形
        builder.move_to(0.0, 1.0);
        builder.line_to(3.0, 1.0);
        builder.line_to(3.0, 2.0);
        builder.line_to(0.0, 2.0);
        builder.close();

        let paths = builder.paths();
        assert_eq!(paths.len(), 2);

        let removed = builder.removed_paths();
        assert!(!removed.is_empty());
    }

    #[test]
    fn test_non_overlapping() {
        let mut builder = OverlapRemoveOutlineBuilder::default();
        // 離れた2つの長方形
        builder.move_to(0.0, 0.0);
        builder.line_to(1.0, 0.0);
        builder.line_to(1.0, 1.0);
        builder.line_to(0.0, 1.0);
        builder.close();
        builder.move_to(5.0, 5.0);
        builder.line_to(6.0, 5.0);
        builder.line_to(6.0, 6.0);
        builder.line_to(5.0, 6.0);
        builder.close();

        let paths = builder.paths();
        assert_eq!(paths.len(), 2);
        let removed = builder.removed_paths();
        assert_eq!(removed.len(), 2);
    }

    #[test]
    fn test_clockwise_detection() {
        // 時計回り（フォント座標系: Y上向き）
        let segments = vec![
            PathSegment::Line(Line {
                from: Point::from_xy(0.0, 0.0),
                to: Point::from_xy(1.0, 0.0),
            }),
            PathSegment::Line(Line {
                from: Point::from_xy(1.0, 0.0),
                to: Point::from_xy(1.0, 1.0),
            }),
            PathSegment::Line(Line {
                from: Point::from_xy(1.0, 1.0),
                to: Point::from_xy(0.0, 1.0),
            }),
            PathSegment::Line(Line {
                from: Point::from_xy(0.0, 1.0),
                to: Point::from_xy(0.0, 0.0),
            }),
        ];
        // フォント座標系では反時計回りの見た目が sum > 0
        assert!(is_clockwise(&segments));
    }

    #[test]
    fn test_turtle_emoji() {
        let font_file = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");
        let face = Face::from_slice(font_file, 0).unwrap();
        let glyph_id = face.glyph_index('🐢').unwrap();
        let mut builder = OverlapRemoveOutlineBuilder::default();
        face.outline_glyph(glyph_id, &mut builder).unwrap();
        let removed = builder.removed_paths();
        assert!(!removed.is_empty());
    }

    #[test]
    fn test_pig_emoji() {
        let font_file = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");
        let face = Face::from_slice(font_file, 0).unwrap();
        let glyph_id = face.glyph_index('🐖').unwrap();
        let mut builder = OverlapRemoveOutlineBuilder::default();
        face.outline_glyph(glyph_id, &mut builder).unwrap();
        let removed = builder.removed_paths();
        assert!(!removed.is_empty());
    }
}
