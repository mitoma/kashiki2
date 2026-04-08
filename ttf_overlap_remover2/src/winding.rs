//! ワインディングナンバー計算
//!
//! ある点が閉じたパスの内側にあるかどうかを判定するために、
//! ワインディングナンバー（巻き数）を計算する。
//!
//! non-zero fill rule: winding number != 0 なら内側

use tiny_skia_path::Point;

use crate::path_segment::PathSegment;

/// サブパスに対するワインディングナンバーを計算する
/// 直線セグメントは直接計算、曲線セグメントはフラット化して近似する
pub(crate) fn winding_number(point: Point, subpath: &[PathSegment]) -> i32 {
    let mut winding = 0i32;

    for segment in subpath {
        match segment {
            PathSegment::Line(line) => {
                winding += crossing_number_line(point, line.from, line.to);
            }
            _ => {
                let points = segment.flatten(0.5);
                for pair in points.windows(2) {
                    winding += crossing_number_line(point, pair[0], pair[1]);
                }
            }
        }
    }

    winding
}

/// 点 p から右方向への半直線が、線分 (v0, v1) を横切る回数を計算する。
/// 下から上に横切る場合は +1、上から下に横切る場合は -1 を返す。
fn crossing_number_line(p: Point, v0: Point, v1: Point) -> i32 {
    if v0.y <= p.y {
        if v1.y > p.y {
            // 上向きの横切り
            if is_left(v0, v1, p) > 0.0 {
                return 1;
            }
        }
    } else if v1.y <= p.y {
        // 下向きの横切り
        if is_left(v0, v1, p) < 0.0 {
            return -1;
        }
    }
    0
}

/// 点 p が線分 (v0, v1) の左側にあるかを判定する。
/// 正の値: 左側、負の値: 右側、0: 線分上
#[inline]
fn is_left(v0: Point, v1: Point, p: Point) -> f32 {
    (v1.x - v0.x) * (p.y - v0.y) - (p.x - v0.x) * (v1.y - v0.y)
}

#[cfg(test)]
mod tests {
    use tiny_skia_path::Point;

    use crate::path_segment::{Line, PathSegment};

    use super::winding_number;

    fn square_path() -> Vec<PathSegment> {
        vec![
            PathSegment::Line(Line {
                from: Point::from_xy(0.0, 0.0),
                to: Point::from_xy(10.0, 0.0),
            }),
            PathSegment::Line(Line {
                from: Point::from_xy(10.0, 0.0),
                to: Point::from_xy(10.0, 10.0),
            }),
            PathSegment::Line(Line {
                from: Point::from_xy(10.0, 10.0),
                to: Point::from_xy(0.0, 10.0),
            }),
            PathSegment::Line(Line {
                from: Point::from_xy(0.0, 10.0),
                to: Point::from_xy(0.0, 0.0),
            }),
        ]
    }

    #[test]
    fn test_point_inside() {
        let path = square_path();
        let w = winding_number(Point::from_xy(5.0, 5.0), &path);
        assert_ne!(w, 0, "中心の点は内側");
    }

    #[test]
    fn test_point_outside() {
        let path = square_path();
        let w = winding_number(Point::from_xy(15.0, 5.0), &path);
        assert_eq!(w, 0, "外側の点は winding = 0");
    }

    #[test]
    fn test_reversed_path() {
        // 逆向きのパスでは winding の符号が反転する
        let path: Vec<PathSegment> = square_path().iter().rev().map(|s| s.reverse()).collect();
        let w = winding_number(Point::from_xy(5.0, 5.0), &path);
        assert_ne!(w, 0, "逆向きでも内側は non-zero");
    }
}
