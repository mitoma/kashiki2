use tiny_skia_path::Point;

pub(crate) fn is_polygon_overlapping(polygon1: &[Point], polygon2: &[Point]) -> bool {
    #[inline]
    fn get_axes(rect: &[Point]) -> Vec<Point> {
        // 各辺に垂直な軸を生成
        (0..rect.len())
            .map(|i| {
                let Point { x: x1, y: y1 } = rect[i];
                let Point { x: x2, y: y2 } = rect[(i + 1) % rect.len()];
                Point {
                    x: y2 - y1,
                    y: x1 - x2,
                } // 法線ベクトル
            })
            .collect()
    }

    #[inline]
    fn project(rect: &[Point], axis: Point) -> Point {
        rect.iter()
            .map(|&Point { x, y }| x * axis.x + y * axis.y)
            .fold(
                Point {
                    x: f32::INFINITY,
                    y: f32::NEG_INFINITY,
                },
                |Point { x: min, y: max }, p| Point {
                    x: min.min(p),
                    y: max.max(p),
                },
            )
    }

    #[inline]
    fn overlap(interval1: Point, interval2: Point) -> bool {
        !(interval1.y < interval2.x || interval2.y < interval1.x)
    }

    let axes1 = get_axes(polygon1);
    let axes2 = get_axes(polygon2);

    // 両方の長方形の軸について区間の重なりを確認
    for axis in axes1.into_iter().chain(axes2) {
        let proj1 = project(polygon1, axis);
        let proj2 = project(polygon2, axis);

        if !overlap(proj1, proj2) {
            return false; // 分離軸が見つかれば重ならない
        }
    }

    true // 全ての軸で重なりがあれば重なっている
}

mod tests {

    use tiny_skia_path::Point;

    use crate::sat::is_polygon_overlapping;

    #[test]
    fn test_polygons_overlap() {
        let polygon1 = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 2.0, y: 0.0 },
            Point { x: 2.0, y: 2.0 },
            Point { x: 0.0, y: 2.0 },
        ];
        let polygon2 = vec![
            Point { x: 1.0, y: 1.0 },
            Point { x: 3.0, y: 1.0 },
            Point { x: 3.0, y: 3.0 },
            Point { x: 1.0, y: 3.0 },
        ];
        assert!(is_polygon_overlapping(&polygon1, &polygon2));
    }

    #[test]
    fn test_polygons_do_not_overlap() {
        let polygon1 = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 2.0, y: 0.0 },
            Point { x: 2.0, y: 2.0 },
            Point { x: 0.0, y: 2.0 },
        ];
        let polygon2 = vec![
            Point { x: 3.0, y: 3.0 },
            Point { x: 5.0, y: 3.0 },
            Point { x: 5.0, y: 5.0 },
            Point { x: 3.0, y: 5.0 },
        ];
        assert!(!is_polygon_overlapping(&polygon1, &polygon2));
    }

    #[test]
    fn test_polygons_touch_but_do_not_overlap() {
        let polygon1 = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 2.0, y: 0.0 },
            Point { x: 2.0, y: 2.0 },
            Point { x: 0.0, y: 2.0 },
        ];
        let polygon2 = vec![
            Point { x: 2.0, y: 0.0 },
            Point { x: 4.0, y: 0.0 },
            Point { x: 4.0, y: 2.0 },
            Point { x: 2.0, y: 2.0 },
        ];
        // 頂点が重なることも重なっているとみなす
        assert!(is_polygon_overlapping(&polygon1, &polygon2));
    }

    #[test]
    fn test_polygons_completely_overlap() {
        let polygon1 = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 4.0, y: 0.0 },
            Point { x: 4.0, y: 4.0 },
            Point { x: 0.0, y: 4.0 },
        ];
        let polygon2 = vec![
            Point { x: 1.0, y: 1.0 },
            Point { x: 3.0, y: 1.0 },
            Point { x: 3.0, y: 3.0 },
            Point { x: 1.0, y: 3.0 },
        ];
        assert!(is_polygon_overlapping(&polygon1, &polygon2));
    }
}
