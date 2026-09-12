use glam::Vec2;

const STRAIGHT_THRESHOLD: f32 = 0.0001;
/// 2次ベジエの1階微分 B'(t)
fn bezier_derivative(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> Vec2 {
    let a = (p1 - p0) * (2.0 * (1.0 - t));
    let b = (p2 - p1) * (2.0 * t);
    a + b
}

/// 2次ベジエの2階微分 B''(t)（t に依存しない）
fn bezier_second_derivative(p0: Vec2, p1: Vec2, p2: Vec2) -> Vec2 {
    (p2 - p1 * 2.0 + p0) * 2.0
}

/// 曲率 κ(t)
fn curvature(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> f32 {
    let d1 = bezier_derivative(p0, p1, p2, t);
    let d2 = bezier_second_derivative(p0, p1, p2);

    // 外積（2D の場合はスカラー）
    let numerator = (d1.x * d2.y - d1.y * d2.x).abs();

    let denom = (d1.length_squared()).powf(1.5);

    if denom < 1e-6 { 0.0 } else { numerator / denom }
}

/// ベジエが「ほぼ直線」かどうか判定する
/// threshold は用途に応じて調整（例: 0.001）
fn is_nearly_straight(p0: Vec2, p1: Vec2, p2: Vec2, threshold: f32) -> bool {
    let mut max_k = 0.0;

    // t をサンプリングして最大曲率を取る
    for i in 0..20 {
        let t = i as f32 / 19.0;
        let k = curvature(p0, p1, p2, t);
        if k > max_k {
            max_k = k;
        }
    }

    max_k < threshold
}

pub(crate) fn is_nearly_straight_default(p0: Vec2, p1: Vec2, p2: Vec2) -> bool {
    is_nearly_straight(p0, p1, p2, STRAIGHT_THRESHOLD)
}

/// move_to 〜 close の間にバッファされる、簡約前の生パスコマンド
pub(crate) enum PendingSegment {
    Line { x: f32, y: f32 },
    Quad { x1: f32, y1: f32, x: f32, y: f32 },
}

// 直線ランの許容誤差。区間長に対する比率としきい値の下限を組み合わせて判定する。
const COLLINEAR_RATIO: f32 = 0.001;
const COLLINEAR_MIN: f32 = 0.01;

/// 点 point と直線 run_start-run_end との距離
fn distance_from_line(run_start: Vec2, run_end: Vec2, point: Vec2) -> f32 {
    let dir = run_end - run_start;
    let len = dir.length();
    if len < 1e-6 {
        return (point - run_start).length();
    }
    let to_point = point - run_start;
    (dir.x * to_point.y - dir.y * to_point.x).abs() / len
}

/// run_start から candidate_end への直線に、points が十分近いか判定する
fn is_collinear(run_start: Vec2, candidate_end: Vec2, points: &[Vec2]) -> bool {
    let len = (candidate_end - run_start).length();
    let threshold = (len * COLLINEAR_RATIO).max(COLLINEAR_MIN);
    points
        .iter()
        .all(|&p| distance_from_line(run_start, candidate_end, p) <= threshold)
}

/// フォントによっては、実質的に直線であるパスを複数の直線・ベジエ曲線に
/// 分割して表現していることがある。単発の quad_to だけでは判定できないため、
/// サブパス単位で溜めたコマンド列を先頭から走査し、連続する「単体でもほぼ直線」な
/// 区間が全体として直線とみなせる場合はまとめて 1 本の line に簡約する。
pub(crate) fn simplify_straight_runs(
    subpath_start: Vec2,
    segments: Vec<PendingSegment>,
) -> Vec<PendingSegment> {
    let mut result = Vec::with_capacity(segments.len());
    let mut run_start = subpath_start;
    let mut run_points: Vec<Vec2> = Vec::new();
    let mut prev = subpath_start;

    for seg in segments {
        let (end, own_straight) = match seg {
            PendingSegment::Line { x, y } => (Vec2::new(x, y), true),
            PendingSegment::Quad { x1, y1, x, y } => {
                let end = Vec2::new(x, y);
                let straight = is_nearly_straight_default(prev, Vec2::new(x1, y1), end);
                (end, straight)
            }
        };

        if own_straight {
            let mut candidate_points = run_points.clone();
            candidate_points.push(prev);
            if is_collinear(run_start, end, &candidate_points) {
                run_points.push(end);
                prev = end;
                continue;
            }
        }

        // ここまでの直線ランを 1 本の直線として確定する
        if let Some(&last) = run_points.last() {
            result.push(PendingSegment::Line {
                x: last.x,
                y: last.y,
            });
            run_start = last;
            run_points.clear();
        }

        if own_straight {
            run_points.push(end);
        } else {
            result.push(seg);
            run_start = end;
        }
        prev = end;
    }

    if let Some(&last) = run_points.last() {
        result.push(PendingSegment::Line {
            x: last.x,
            y: last.y,
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simplify_straight_runs_merges_collinear_lines() {
        let segments = vec![
            PendingSegment::Line { x: 5.0, y: 0.0 },
            PendingSegment::Line { x: 10.0, y: 0.0 },
            PendingSegment::Line { x: 10.0, y: 10.0 },
        ];
        let simplified = simplify_straight_runs(Vec2::new(0.0, 0.0), segments);

        // 最初の 2 区間（0,0)->(5,0)->(10,0) は同一直線上なので 1 本に簡約される
        assert_eq!(simplified.len(), 2);
        match simplified[0] {
            PendingSegment::Line { x, y } => {
                assert_eq!((x, y), (10.0, 0.0));
            }
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn simplify_straight_runs_merges_nearly_straight_quads() {
        // 制御点が両端点を結ぶ直線上にあるため、曲率 0 の「見せかけの曲線」になる
        let segments = vec![
            PendingSegment::Quad {
                x1: 2.5,
                y1: 0.0,
                x: 5.0,
                y: 0.0,
            },
            PendingSegment::Quad {
                x1: 7.5,
                y1: 0.0,
                x: 10.0,
                y: 0.0,
            },
        ];
        let simplified = simplify_straight_runs(Vec2::new(0.0, 0.0), segments);

        assert_eq!(simplified.len(), 1);
        match simplified[0] {
            PendingSegment::Line { x, y } => assert_eq!((x, y), (10.0, 0.0)),
            _ => panic!("expected the two quads to collapse into a single line"),
        }
    }

    #[test]
    fn simplify_straight_runs_keeps_real_curves() {
        let segments = vec![PendingSegment::Quad {
            x1: 5.0,
            y1: 10.0,
            x: 10.0,
            y: 0.0,
        }];
        let simplified = simplify_straight_runs(Vec2::new(0.0, 0.0), segments);

        assert_eq!(simplified.len(), 1);
        assert!(matches!(simplified[0], PendingSegment::Quad { .. }));
    }
}
