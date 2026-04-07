//! フラットパス関連のユーティリティ（将来の拡張用）

use tiny_skia_path::Point;

use crate::path_segment::PathSegment;

/// セグメント列をフラット化して頂点列に変換する
#[allow(dead_code)]
pub(crate) fn flatten_segments(segments: &[PathSegment], tolerance: f32) -> Vec<Point> {
    let mut points = Vec::new();
    for (i, seg) in segments.iter().enumerate() {
        let flat = seg.flatten(tolerance);
        if i == 0 {
            points.extend_from_slice(&flat);
        } else {
            // 最初の点は前のセグメントの終点と重複するのでスキップ
            points.extend_from_slice(&flat[1..]);
        }
    }
    points
}
