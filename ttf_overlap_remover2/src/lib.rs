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
#[cfg(test)]
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

/// 全サブパスの和集合を求めてオーバーラップを除去する。
///
/// non-zero winding fill rule で設計されたパスを、
/// even-odd fill rule でも同じ見た目になるように変換する。
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

    // 全セグメントペア間の交差点を検出して分割する
    all_segments = split_all_segments(all_segments);

    // 各エッジの左右のワインディングナンバーを計算して境界エッジのみ残す
    let boundary_segments = filter_boundary_segments(&all_segments, &all_subpaths);

    if boundary_segments.is_empty() {
        return paths;
    }

    // セグメントからループを再構成
    let loops = build_loops(&boundary_segments);

    // Path に変換
    let result: Vec<Path> = loops
        .iter()
        .filter_map(|loop_seg| segments_to_path(loop_seg))
        .collect();

    if result.is_empty() {
        return paths;
    }
    result
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

/// 全セグメントペア間で交差点検出・分割を行う
fn split_all_segments(segments: Vec<PathSegment>) -> Vec<PathSegment> {
    let mut result: Vec<PathSegment> = segments;

    let mut changed = true;
    while changed {
        changed = false;
        let mut i = 0;
        while i < result.len() {
            let mut j = i + 1;
            while j < result.len() {
                if result[i].is_same_or_reversed(&result[j]) {
                    j += 1;
                    continue;
                }

                let cross_points = cross_point::find_cross_points(&result[i], &result[j]);
                if cross_points.is_empty() {
                    j += 1;
                    continue;
                }

                // 分割する
                let seg_i = result.remove(i);
                let j_adj = if j > i { j - 1 } else { j };
                let seg_j = result.remove(j_adj);

                let split_i = split_segment_at_points(&seg_i, &cross_points, true);
                let split_j = split_segment_at_points(&seg_j, &cross_points, false);

                let insert_pos = i.min(j_adj);
                for (k, s) in split_i.into_iter().chain(split_j.into_iter()).enumerate() {
                    result.insert(insert_pos + k, s);
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
    result
}

/// 交差点でセグメントを分割する
fn split_segment_at_points(
    segment: &PathSegment,
    cross_points: &[cross_point::CrossPoint],
    is_a: bool,
) -> Vec<PathSegment> {
    // t値と対応する交差点座標をペアにする
    let mut positions: Vec<(f32, Point)> = cross_points
        .iter()
        .filter_map(|cp| {
            let t = if is_a { cp.t_a } else { cp.t_b };
            if t > EPSILON && t < 1.0 - EPSILON {
                Some((t, cp.point))
            } else {
                None
            }
        })
        .collect();
    positions.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    positions.dedup_by(|a, b| {
        let dup = (a.0 - b.0).abs() < EPSILON;
        if dup {
            // b を残す（dedup_by は b を残す）
        }
        dup
    });

    if positions.is_empty() {
        return vec![segment.clone()];
    }

    let mut result = Vec::new();
    let mut remaining = segment.clone();
    let mut consumed = 0.0f32;

    for &(t, canonical_point) in &positions {
        let adjusted_t = (t - consumed) / (1.0 - consumed);
        if adjusted_t <= EPSILON || adjusted_t >= 1.0 - EPSILON {
            continue;
        }
        let (pre, post) = remaining.chop(adjusted_t);

        // 交差点の座標は CrossPoint.point を使う（両方のセグメントで共有）
        let mut pre = pre;
        let mut post = post;
        pre.set_to(canonical_point);
        post.set_from(canonical_point);

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
const GREEDY_EPSILON: f32 = 0.5;

/// 各エッジの左右のワインディングナンバーを計算し、境界エッジのみ残す。
///
/// 境界エッジとは: エッジの左側が外側(winding==0)で右側が内側(winding!=0)のエッジ。
/// これにより正しい巻き方向のエッジのみ残り、一意にループが構成できる。
fn filter_boundary_segments(
    segments: &[PathSegment],
    original_subpaths: &[Vec<PathSegment>],
) -> Vec<PathSegment> {
    segments
        .iter()
        .filter(|seg| {
            let mid = seg.evaluate(0.5);
            let tangent = compute_tangent(seg);
            let len = (tangent.x * tangent.x + tangent.y * tangent.y).sqrt();
            if len < 1e-10 {
                return false; // 退化セグメントは除去
            }
            let tx = tangent.x / len;
            let ty = tangent.y / len;

            // セグメントの長さに応じたオフセット量
            let (from, to) = seg.endpoints();
            let seg_len = from.distance(to);
            let offset = (seg_len * 0.01).clamp(0.1, 2.0);

            // 左法線 (進行方向から見て左 = 反時計回り90度回転)
            let left = Point::from_xy(mid.x - ty * offset, mid.y + tx * offset);
            // 右法線
            let right = Point::from_xy(mid.x + ty * offset, mid.y - tx * offset);

            let w_left = total_winding(left, original_subpaths);
            let w_right = total_winding(right, original_subpaths);

            // 片方が外側(0)で他方が内側(非0): 境界エッジ
            // outer boundary: w_left=0, w_right≠0
            // hole boundary: w_left≠0, w_right=0
            (w_left == 0) != (w_right == 0)
        })
        .cloned()
        .collect()
}

/// セグメントの中点付近の接線ベクトルを計算
fn compute_tangent(seg: &PathSegment) -> Point {
    let p0 = seg.evaluate(0.49);
    let p1 = seg.evaluate(0.51);
    Point::from_xy(p1.x - p0.x, p1.y - p0.y)
}

/// 全サブパスに対するワインディングナンバーの合計
fn total_winding(point: Point, subpaths: &[Vec<PathSegment>]) -> i32 {
    let mut total = 0i32;
    for subpath in subpaths {
        total += winding::winding_number(point, subpath);
    }
    total
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
            if path.len() > 1 && to.distance(start_from) < GREEDY_EPSILON {
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
            let mut best_dist = GREEDY_EPSILON;
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
    use tiny_skia_path::{Path, PathBuilder, Point};

    use crate::{
        OverlapRemoveOutlineBuilder, is_clockwise,
        path_segment::{Line, PathSegment},
    };

    /// 複数の Path を 1 つの Path にまとめる
    fn combine_paths(paths: &[Path]) -> Option<Path> {
        let mut pb = PathBuilder::new();
        for path in paths {
            for seg in path.segments() {
                match seg {
                    tiny_skia_path::PathSegment::MoveTo(p) => pb.move_to(p.x, p.y),
                    tiny_skia_path::PathSegment::LineTo(p) => pb.line_to(p.x, p.y),
                    tiny_skia_path::PathSegment::QuadTo(c, p) => pb.quad_to(c.x, c.y, p.x, p.y),
                    tiny_skia_path::PathSegment::CubicTo(c1, c2, p) => {
                        pb.cubic_to(c1.x, c1.y, c2.x, c2.y, p.x, p.y)
                    }
                    tiny_skia_path::PathSegment::Close => pb.close(),
                }
            }
        }
        pb.finish()
    }

    #[test]
    fn test_cross_shape() {
        // 2つの菱形が部分的に重なるテスト（共線セグメントなし）
        let mut builder = OverlapRemoveOutlineBuilder::default();
        // 菱形1: 中心(5,5)
        builder.move_to(5.0, 0.0);
        builder.line_to(10.0, 5.0);
        builder.line_to(5.0, 10.0);
        builder.line_to(0.0, 5.0);
        builder.close();
        // 菱形2: 中心(8,5)、右にずらして重なる
        builder.move_to(8.0, 0.0);
        builder.line_to(13.0, 5.0);
        builder.line_to(8.0, 10.0);
        builder.line_to(3.0, 5.0);
        builder.close();

        let paths = builder.paths();
        assert_eq!(paths.len(), 2);

        let removed = builder.removed_paths();
        assert!(!removed.is_empty());

        // 重要: 除去後の paths を EvenOdd で描画した結果が、
        // 元の paths を Winding で描画した結果と一致すること
        use tiny_skia::{Color, Paint, Pixmap, Transform};
        let canvas_size = 100u32;
        let scale = canvas_size as f32 / 14.0;
        let transform =
            Transform::from_scale(scale, -scale).post_translate(10.0, canvas_size as f32 - 10.0);

        let render = |paths: &[tiny_skia_path::Path], fill_rule: tiny_skia::FillRule| -> Pixmap {
            let mut pixmap = Pixmap::new(canvas_size, canvas_size).unwrap();
            let mut paint = Paint {
                anti_alias: false,
                ..Paint::default()
            };
            pixmap.fill(Color::WHITE);
            // 全パスを1つにまとめて描画（アプリのシェーダと同じ動作）
            let combined = combine_paths(paths);
            if let Some(ref path) = combined {
                paint.set_color_rgba8(0, 0, 0, 255);
                pixmap.fill_path(path, &paint, fill_rule, transform, None);
            }
            pixmap
        };

        let original_winding = render(&paths, tiny_skia::FillRule::Winding);
        let removed_evenodd = render(&removed, tiny_skia::FillRule::EvenOdd);

        let diff_count = original_winding
            .pixels()
            .iter()
            .zip(removed_evenodd.pixels())
            .filter(|(w, e)| w != e)
            .count();

        // 元パスの Winding vs EvenOdd に差があることを確認
        // （差がなければオーバーラップ除去のテストとして意味がない）
        let original_evenodd = render(&paths, tiny_skia::FillRule::EvenOdd);
        let original_diff = original_winding
            .pixels()
            .iter()
            .zip(original_evenodd.pixels())
            .filter(|(w, e)| w != e)
            .count();
        eprintln!(
            "十字型: 元パスの Winding vs EvenOdd 差分: {} pixels",
            original_diff
        );
        eprintln!(
            "十字型: 除去後(EvenOdd) vs 元(Winding) 差分: {} pixels",
            diff_count
        );
        assert!(
            original_diff > 0,
            "テストケースにオーバーラップが存在しない"
        );
        assert!(
            diff_count == 0,
            "オーバーラップ除去後、EvenOdd で描画した結果が元の Winding と一致しない: {} pixels 差分",
            diff_count,
        );
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

    /// ピクセル比較による品質テスト
    /// 元のパスを Winding で描画した結果と、overlap 除去後を EvenOdd で描画した結果を比較する。
    /// オーバーラップ除去の目的は、Winding 前提の元パスを EvenOdd でも同じ見た目にすること。
    ///
    /// 注: アプリのシェーダは全サブパスをまとめて Even-Odd 判定するため、
    /// テストでも全サブパスを1つの Path にまとめて描画する。
    fn pixel_compare_emoji(c: char) -> f32 {
        use tiny_skia::{Color, Paint, Pixmap, Transform};

        let font_file = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");
        let face = Face::from_slice(font_file, 0).unwrap();
        let glyph_id = face.glyph_index(c).unwrap();
        let mut builder = OverlapRemoveOutlineBuilder::default();
        face.outline_glyph(glyph_id, &mut builder).unwrap();

        let original_paths = builder.paths();
        let removed_paths = builder.removed_paths();

        let canvas_size = 500u32;
        let scale = canvas_size as f32 / 1100.0;
        let transform =
            Transform::from_scale(scale, -scale).post_translate(50.0, canvas_size as f32 - 50.0);

        let render = |paths: &[tiny_skia_path::Path], fill_rule: tiny_skia::FillRule| -> Pixmap {
            let mut pixmap = Pixmap::new(canvas_size, canvas_size).unwrap();
            let mut paint = Paint {
                anti_alias: false,
                ..Paint::default()
            };
            pixmap.fill(Color::WHITE);
            // 全パスを1つにまとめて描画（アプリのシェーダと同じ動作）
            let combined = combine_paths(paths);
            if let Some(ref path) = combined {
                paint.set_color_rgba8(0, 0, 0, 255);
                pixmap.fill_path(path, &paint, fill_rule, transform, None);
            }
            pixmap
        };

        // 期待値: 元パスを Winding で描画 → 正しい見た目
        let original = render(&original_paths, tiny_skia::FillRule::Winding);
        // テスト対象: 除去後パスを EvenOdd で描画
        let removed = render(&removed_paths, tiny_skia::FillRule::EvenOdd);

        let (total, equal) = original.pixels().iter().zip(removed.pixels()).fold(
            (0u64, 0u64),
            |(total, equal), (o, r)| {
                if o == r {
                    (total + 1, equal + 1)
                } else {
                    (total + 1, equal)
                }
            },
        );

        equal as f32 / total as f32
    }

    #[test]
    fn test_turtle_quality() {
        let rate = pixel_compare_emoji('🐢');
        eprintln!("🐢 一致率: {}", rate);
        assert!(rate > 0.99, "🐢 一致率が低い: {}", rate);
    }

    #[test]
    fn test_dog_quality() {
        let rate = pixel_compare_emoji('🐕');
        eprintln!("🐕 一致率: {}", rate);
        assert!(rate > 0.99, "🐕 一致率が低い: {}", rate);
    }

    #[test]
    fn test_kadomatsu_quality() {
        let rate = pixel_compare_emoji('🎍');
        eprintln!("🎍 一致率: {}", rate);
        assert!(rate > 0.99, "🎍 一致率が低い: {}", rate);
    }

    #[test]
    fn test_pig_quality() {
        let rate = pixel_compare_emoji('🐖');
        eprintln!("🐖 一致率: {}", rate);
        assert!(rate > 0.99, "🐖 一致率が低い: {}", rate);
    }

    #[test]
    fn test_wave_quality() {
        let rate = pixel_compare_emoji('🌊');
        eprintln!("🌊 一致率: {}", rate);
        assert!(rate > 0.99, "🌊 一致率が低い: {}", rate);
    }

    #[test]
    fn test_elephant_quality() {
        let rate = pixel_compare_emoji('🐘');
        eprintln!("🐘 一致率: {}", rate);
        assert!(rate > 0.99, "🐘 一致率が低い: {}", rate);
    }

    #[test]
    fn test_mountain_quality() {
        let rate = pixel_compare_emoji('🏔');
        eprintln!("🏔 一致率: {}", rate);
        assert!(rate > 0.99, "🏔 一致率が低い: {}", rate);
    }

    #[test]
    fn test_cityscape_quality() {
        let rate = pixel_compare_emoji('🏙');
        eprintln!("🏙 一致率: {}", rate);
        assert!(rate > 0.99, "🏙 一致率が低い: {}", rate);
    }

    /// NotoEmoji のグリフで Winding vs EvenOdd の差異があるものを探す
    #[test]
    fn test_find_overlap_needed_glyphs() {
        use tiny_skia::{Color, Paint, Pixmap, Transform};

        let font_file = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");
        let face = Face::from_slice(font_file, 0).unwrap();

        let canvas_size = 200u32;
        let scale = canvas_size as f32 / 1100.0;
        let transform =
            Transform::from_scale(scale, -scale).post_translate(20.0, canvas_size as f32 - 20.0);

        // テスト用絵文字のリスト
        let test_chars = [
            '🐢', '🐕', '🎍', '🐖', '🌊', '🐄', '⛩', '🍖', '🗻', '🎋', '🌅', '🏔', '🐘', '🐎', '🐑',
            '🦁', '🐻', '🐔', '🐙', '🦀', '🦊', '🐿', '🦢',
        ];

        let mut needs_removal = Vec::new();

        for c in test_chars {
            let Some(glyph_id) = face.glyph_index(c) else {
                continue;
            };
            let mut builder = OverlapRemoveOutlineBuilder::default();
            if face.outline_glyph(glyph_id, &mut builder).is_none() {
                continue;
            }
            let paths = builder.paths();

            let render = |fill_rule: tiny_skia::FillRule| -> Pixmap {
                let mut pixmap = Pixmap::new(canvas_size, canvas_size).unwrap();
                let mut paint = Paint {
                    anti_alias: false,
                    ..Paint::default()
                };
                pixmap.fill(Color::WHITE);
                let combined = combine_paths(&paths);
                if let Some(ref path) = combined {
                    paint.set_color_rgba8(0, 0, 0, 255);
                    pixmap.fill_path(path, &paint, fill_rule, transform, None);
                }
                pixmap
            };

            let winding = render(tiny_skia::FillRule::Winding);
            let evenodd = render(tiny_skia::FillRule::EvenOdd);

            let diff_count = winding
                .pixels()
                .iter()
                .zip(evenodd.pixels())
                .filter(|(w, e)| w != e)
                .count();

            if diff_count > 0 {
                let total = (canvas_size * canvas_size) as usize;
                needs_removal.push((c, diff_count, total));
                eprintln!(
                    "{} (U+{:04X}): Winding vs EvenOdd differs in {} / {} pixels ({:.1}%)",
                    c,
                    c as u32,
                    diff_count,
                    total,
                    diff_count as f64 / total as f64 * 100.0,
                );
            }
        }

        eprintln!(
            "\nオーバーラップ除去が必要なグリフ: {} / {}",
            needs_removal.len(),
            test_chars.len()
        );
        // このテスト自体は失敗しない（情報収集目的）
    }
}
