use bezier_converter::CubicBezier;
use glam::Vec2;
use log::debug;
use skrifa::outline::OutlinePen;

use crate::straight_run_simplifier::{PendingSegment, simplify_straight_runs};

struct Subpath {
    start: [f32; 2],
    segments: Vec<PendingSegment>,
}

struct CurrentSubpath {
    start: [f32; 2],
    segments: Vec<PendingSegment>,
    last_point: [f32; 2],
}

/// フォントによっては、実質的に直線であるパスを複数のベジエ曲線に分割して
/// 表現していることがある。`OverlapRemoveOutlineBuilder` と同様にデコレーターとして
/// アウトラインをいったん蓄積し、サブパスごとに直線ランを簡約してから
/// 別の `OutlinePen`（`VectorVertexBuilder` 等）へ流し込む。
#[derive(Default)]
pub struct StraightenOutlineBuilder {
    subpaths: Vec<Subpath>,
    current: Option<CurrentSubpath>,
}

impl StraightenOutlineBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// 簡約後のアウトラインを別の OutlinePen へ流し込む
    pub fn outline<T: OutlinePen>(&self, builder: &mut T) {
        for subpath in &self.subpaths {
            builder.move_to(subpath.start[0], subpath.start[1]);
            for seg in &subpath.segments {
                match *seg {
                    PendingSegment::Line { x, y } => builder.line_to(x, y),
                    PendingSegment::Quad { x1, y1, x, y } => builder.quad_to(x1, y1, x, y),
                }
            }
            builder.close();
        }
    }
}

impl OutlinePen for StraightenOutlineBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        // close() されずに次の move_to が呼ばれた場合の保険として、未確定のまま破棄せず取り込む
        if let Some(cur) = self.current.take() {
            self.subpaths.push(Subpath {
                start: cur.start,
                segments: cur.segments,
            });
        }
        self.current = Some(CurrentSubpath {
            start: [x, y],
            segments: Vec::new(),
            last_point: [x, y],
        });
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let Some(cur) = &mut self.current else {
            return;
        };
        if cur.last_point == [x, y] {
            return;
        }
        cur.segments.push(PendingSegment::Line { x, y });
        cur.last_point = [x, y];
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let Some(cur) = &mut self.current else {
            return;
        };
        let [px, py] = cur.last_point;
        if px == cx0 && py == cy0 && px == x && py == y {
            return;
        }
        cur.segments.push(PendingSegment::Quad {
            x1: cx0,
            y1: cy0,
            x,
            y,
        });
        cur.last_point = [x, y];
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        // 3 次ベジエを 2 次ベジエに近似してから溜める
        let Some(cur) = &self.current else {
            return;
        };
        let [last_x, last_y] = cur.last_point;
        if last_x == cx0
            && last_y == cy0
            && last_x == cx1
            && last_y == cy1
            && last_x == x
            && last_y == y
        {
            return;
        }

        let cb = CubicBezier {
            x0: last_x,
            y0: last_y,
            x1: x,
            y1: y,
            cx0,
            cy0,
            cx1,
            cy1,
        };
        let qbs = cb.to_quadratic();
        debug!("cubic to quadratic: 1 -> {}", qbs.len());
        for qb in qbs.iter() {
            self.quad_to(qb.cx0, qb.cy0, qb.x1, qb.y1);
        }
    }

    fn close(&mut self) {
        let Some(cur) = self.current.take() else {
            return;
        };
        let mut segments = cur.segments;
        // 終点から始点へ戻る閉じ線も簡約対象に含める
        if cur.last_point != cur.start {
            segments.push(PendingSegment::Line {
                x: cur.start[0],
                y: cur.start[1],
            });
        }
        let simplified = simplify_straight_runs(Vec2::from(cur.start), segments);
        self.subpaths.push(Subpath {
            start: cur.start,
            segments: simplified,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_vertex::VectorVertexBuilder;

    #[test]
    fn merges_straight_quad_run_across_multiple_segments() {
        let mut straighten = StraightenOutlineBuilder::new();
        straighten.move_to(0.0, 0.0);
        // 制御点が両端点を結ぶ直線上にあるため、それぞれは曲率 0 の「見せかけの曲線」
        straighten.quad_to(2.5, 0.0, 5.0, 0.0);
        straighten.quad_to(7.5, 0.0, 10.0, 0.0);
        straighten.line_to(10.0, 10.0);
        straighten.close();

        let mut builder = VectorVertexBuilder::new();
        straighten.outline(&mut builder);

        // 2 つの quad_to が 1 本の直線に簡約されるので、ベジエ曲線用の Control 頂点は生成されない
        assert!(!builder.has_control_vertex_for_test());
    }
}
