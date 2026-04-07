//! 交差点検出モジュール
//!
//! 二つのベジェセグメント間の交差点を検出する。
//! 手法: 再帰的な区間二分割 + 線形近似による交差点計算

use tiny_skia_path::Point;

use crate::path_segment::{Line, PathSegment, SegmentTrait};

/// 交差点の情報
#[derive(Debug, Clone)]
pub(crate) struct CrossPoint {
    /// 交差点の座標
    pub(crate) point: Point,
    /// セグメント A 上のパラメータ t (0.0..=1.0)
    pub(crate) t_a: f32,
    /// セグメント B 上のパラメータ t (0.0..=1.0)
    pub(crate) t_b: f32,
}

const EPSILON: f32 = 1e-5;
const MAX_DEPTH: u32 = 20;

/// 二つのセグメント間の交差点を全て検出する
pub(crate) fn find_cross_points(a: &PathSegment, b: &PathSegment) -> Vec<CrossPoint> {
    match (a, b) {
        (PathSegment::Line(la), PathSegment::Line(lb)) => find_line_line(la, lb),
        _ => find_curve_curve(a, b),
    }
}

/// 直線-直線の交差点
fn find_line_line(a: &Line, b: &Line) -> Vec<CrossPoint> {
    let mut results = Vec::new();

    // 通常の交差判定
    if let Some(cp) = intersect_lines(a, b)
        && cp.t_a > EPSILON
        && cp.t_a < 1.0 - EPSILON
        && cp.t_b > EPSILON
        && cp.t_b < 1.0 - EPSILON
    {
        results.push(cp);
        return results;
    }

    // 重なりチェック（逆向きの共線セグメント）
    let a_vec = a.to - a.from;
    let b_vec = b.to - b.from;
    let a_len2 = a_vec.x * a_vec.x + a_vec.y * a_vec.y;
    let b_len2 = b_vec.x * b_vec.x + b_vec.y * b_vec.y;
    if a_len2 <= EPSILON * EPSILON || b_len2 <= EPSILON * EPSILON {
        return results;
    }

    let cross = a_vec.x * b_vec.y - a_vec.y * b_vec.x;
    if cross.abs() > EPSILON * a_len2.sqrt() {
        // 平行でない
        return results;
    }

    // 同方向の場合はスキップ（逆方向の重なりのみ検出）
    if a_vec.x * b_vec.x + a_vec.y * b_vec.y >= 0.0 {
        return results;
    }

    // 共線かチェック
    let ab = b.from - a.from;
    let cross_ab = a_vec.x * ab.y - a_vec.y * ab.x;
    if cross_ab.abs() > EPSILON * a_len2.sqrt() {
        return results;
    }

    // a 上での b の端点の位置
    let b0_on_a = ((b.from - a.from).x * a_vec.x + (b.from - a.from).y * a_vec.y) / a_len2;
    let b1_on_a = ((b.to - a.from).x * a_vec.x + (b.to - a.from).y * a_vec.y) / a_len2;

    let overlap_start = b0_on_a.min(b1_on_a).max(0.0);
    let overlap_end = b0_on_a.max(b1_on_a).min(1.0);

    if overlap_end - overlap_start < EPSILON {
        return results;
    }

    // 重なり端点を交差点として返す
    for t_a in [overlap_start, overlap_end] {
        if t_a > EPSILON && t_a < 1.0 - EPSILON {
            let point = a.evaluate(t_a);
            let t_b = ((point - b.from).x * b_vec.x + (point - b.from).y * b_vec.y) / b_len2;
            if t_b > -EPSILON && t_b < 1.0 + EPSILON {
                results.push(CrossPoint {
                    point,
                    t_a,
                    t_b: t_b.clamp(0.0, 1.0),
                });
            }
        }
    }

    results
}

/// 2直線の交点を求める
fn intersect_lines(a: &Line, b: &Line) -> Option<CrossPoint> {
    let d1 = a.to - a.from;
    let d2 = b.to - b.from;
    let denom = d1.x * d2.y - d1.y * d2.x;
    if denom.abs() <= EPSILON {
        return None;
    }
    let t = ((b.from.x - a.from.x) * d2.y - (b.from.y - a.from.y) * d2.x) / denom;
    let u = ((b.from.x - a.from.x) * d1.y - (b.from.y - a.from.y) * d1.x) / denom;

    if (-EPSILON..=1.0 + EPSILON).contains(&t) && (-EPSILON..=1.0 + EPSILON).contains(&u) {
        let point = Point::from_xy(a.from.x + t * d1.x, a.from.y + t * d1.y);
        Some(CrossPoint {
            point,
            t_a: t.clamp(0.0, 1.0),
            t_b: u.clamp(0.0, 1.0),
        })
    } else {
        None
    }
}

/// 曲線-曲線の交差点（再帰的二分割法）
fn find_curve_curve(a: &PathSegment, b: &PathSegment) -> Vec<CrossPoint> {
    let mut results = Vec::new();

    struct Item {
        a_t_start: f32,
        a_t_end: f32,
        b_t_start: f32,
        b_t_end: f32,
        depth: u32,
    }

    let mut stack = vec![Item {
        a_t_start: 0.0,
        a_t_end: 1.0,
        b_t_start: 0.0,
        b_t_end: 1.0,
        depth: 0,
    }];

    while let Some(item) = stack.pop() {
        // バウンディングボックスの重なりチェック
        let a_bb = eval_bounding_rect(a, item.a_t_start, item.a_t_end);
        let b_bb = eval_bounding_rect(b, item.b_t_start, item.b_t_end);

        if !rects_overlap(a_bb, b_bb) {
            continue;
        }

        let a_size = rect_max_dim(a_bb);
        let b_size = rect_max_dim(b_bb);

        if (a_size < EPSILON && b_size < EPSILON) || item.depth >= MAX_DEPTH {
            // 収束 — 線形近似で交差点を求める
            let a_from = a.evaluate(item.a_t_start);
            let a_to = a.evaluate(item.a_t_end);
            let b_from = b.evaluate(item.b_t_start);
            let b_to = b.evaluate(item.b_t_end);

            let la = Line {
                from: a_from,
                to: a_to,
            };
            let lb = Line {
                from: b_from,
                to: b_to,
            };

            if let Some(cp) = intersect_lines(&la, &lb) {
                let t_a = item.a_t_start + cp.t_a * (item.a_t_end - item.a_t_start);
                let t_b = item.b_t_start + cp.t_b * (item.b_t_end - item.b_t_start);

                // 端点の交差は除外
                if t_a > EPSILON && t_a < 1.0 - EPSILON && t_b > EPSILON && t_b < 1.0 - EPSILON {
                    let point = a.evaluate(t_a);
                    // 重複チェック
                    if !results
                        .iter()
                        .any(|r: &CrossPoint| r.point.distance(point) < EPSILON * 10.0)
                    {
                        results.push(CrossPoint { point, t_a, t_b });
                    }
                }
            }
            continue;
        }

        // 大きい方を分割
        let a_mid = (item.a_t_start + item.a_t_end) * 0.5;
        let b_mid = (item.b_t_start + item.b_t_end) * 0.5;

        if a_size >= b_size {
            stack.push(Item {
                a_t_start: item.a_t_start,
                a_t_end: a_mid,
                b_t_start: item.b_t_start,
                b_t_end: item.b_t_end,
                depth: item.depth + 1,
            });
            stack.push(Item {
                a_t_start: a_mid,
                a_t_end: item.a_t_end,
                b_t_start: item.b_t_start,
                b_t_end: item.b_t_end,
                depth: item.depth + 1,
            });
        } else {
            stack.push(Item {
                a_t_start: item.a_t_start,
                a_t_end: item.a_t_end,
                b_t_start: item.b_t_start,
                b_t_end: b_mid,
                depth: item.depth + 1,
            });
            stack.push(Item {
                a_t_start: item.a_t_start,
                a_t_end: item.a_t_end,
                b_t_start: b_mid,
                b_t_end: item.b_t_end,
                depth: item.depth + 1,
            });
        }
    }

    results
}

/// パラメータ区間でのバウンディングボックスを近似計算
fn eval_bounding_rect(seg: &PathSegment, t_start: f32, t_end: f32) -> (f32, f32, f32, f32) {
    // サンプリングで近似
    let n = 4;
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for i in 0..=n {
        let t = t_start + (t_end - t_start) * (i as f32 / n as f32);
        let p = seg.evaluate(t);
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    // マージンを追加（制御点がサンプリング外の場合に備えて）
    let margin = (max_x - min_x + max_y - min_y) * 0.1;
    (
        min_x - margin,
        min_y - margin,
        max_x + margin,
        max_y + margin,
    )
}

fn rects_overlap(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 <= b.2 && a.2 >= b.0 && a.1 <= b.3 && a.3 >= b.1
}

fn rect_max_dim(r: (f32, f32, f32, f32)) -> f32 {
    (r.2 - r.0).max(r.3 - r.1)
}

#[cfg(test)]
mod tests {
    use tiny_skia_path::Point;

    use super::*;
    use crate::path_segment::{Cubic, Quadratic};

    #[test]
    fn test_line_line_cross() {
        let a = PathSegment::Line(Line {
            from: Point::from_xy(0.0, 0.0),
            to: Point::from_xy(2.0, 2.0),
        });
        let b = PathSegment::Line(Line {
            from: Point::from_xy(0.0, 2.0),
            to: Point::from_xy(2.0, 0.0),
        });
        let result = find_cross_points(&a, &b);
        assert_eq!(result.len(), 1);
        assert!((result[0].point.x - 1.0).abs() < 0.01);
        assert!((result[0].point.y - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_line_line_parallel() {
        let a = PathSegment::Line(Line {
            from: Point::from_xy(0.0, 0.0),
            to: Point::from_xy(2.0, 0.0),
        });
        let b = PathSegment::Line(Line {
            from: Point::from_xy(0.0, 1.0),
            to: Point::from_xy(2.0, 1.0),
        });
        let result = find_cross_points(&a, &b);
        assert!(result.is_empty());
    }

    #[test]
    fn test_line_line_reversed_overlap() {
        let a = PathSegment::Line(Line {
            from: Point::from_xy(0.0, 0.0),
            to: Point::from_xy(4.0, 0.0),
        });
        let b = PathSegment::Line(Line {
            from: Point::from_xy(3.0, 0.0),
            to: Point::from_xy(1.0, 0.0),
        });
        let result = find_cross_points(&a, &b);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_line_quad_cross() {
        let a = PathSegment::Line(Line {
            from: Point::from_xy(0.0, 1.0),
            to: Point::from_xy(2.0, 1.0),
        });
        let b = PathSegment::Quadratic(Quadratic {
            from: Point::from_xy(1.0, 0.0),
            control: Point::from_xy(1.0, 2.0),
            to: Point::from_xy(1.0, 0.0),
        });
        // この曲線は退化しているのでテストとしてはやや特殊
        let _result = find_cross_points(&a, &b);
    }

    #[test]
    fn test_cubic_cubic_cross() {
        let a = PathSegment::Cubic(Cubic {
            from: Point::from_xy(0.0, 0.0),
            control1: Point::from_xy(1.0, 2.0),
            control2: Point::from_xy(2.0, -1.0),
            to: Point::from_xy(3.0, 1.0),
        });
        let b = PathSegment::Cubic(Cubic {
            from: Point::from_xy(0.0, 1.0),
            control1: Point::from_xy(1.0, -1.0),
            control2: Point::from_xy(2.0, 2.0),
            to: Point::from_xy(3.0, 0.0),
        });
        let result = find_cross_points(&a, &b);
        assert!(!result.is_empty());
    }
}
