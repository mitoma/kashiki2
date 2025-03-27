use std::cmp::Ordering;

use tiny_skia_path::{NormalizedF32Exclusive, Point, Rect, path_geometry};

use crate::cmp_clockwise;

// PathSegment に備わっていてほしい共通操作を集めた trait
pub(crate) trait SegmentTrait
where
    Self: Sized + PartialEq + Clone,
{
    #[allow(dead_code)]
    fn move_to(&self, point: Point) -> Self;
    fn set_from(&mut self, point: Point);
    fn set_to(&mut self, point: Point);
    fn endpoints(&self) -> (Point, Point);
    fn rect(&self) -> Rect;
    fn chop_harf(&self) -> (Self, Self);
    fn chop(&self, position: f32) -> (Self, Self);
    #[allow(dead_code)]
    fn to_path_segment(self) -> PathSegment;
    fn reverse(&self) -> Self;
    fn is_same_or_reversed(&self, other: &Self) -> bool;
    #[allow(clippy::wrong_self_convention)]
    fn from_vector(&self) -> Point;
    fn to_vector(&self) -> Point;
    fn polygon(&self) -> Vec<Point>;
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Line {
    pub(crate) from: Point,
    pub(crate) to: Point,
}

impl SegmentTrait for Line {
    fn move_to(&self, point: Point) -> Self {
        Line {
            from: self.from + point,
            to: self.to + point,
        }
    }

    fn set_from(&mut self, point: Point) {
        self.from = point;
    }

    fn set_to(&mut self, point: Point) {
        self.to = point;
    }

    fn endpoints(&self) -> (Point, Point) {
        (self.from, self.to)
    }

    fn rect(&self) -> Rect {
        let min_x = self.from.x.min(self.to.x);
        let min_y = self.from.y.min(self.to.y);
        let max_x = self.from.x.max(self.to.x);
        let max_y = self.from.y.max(self.to.y);
        Rect::from_xywh(min_x, min_y, max_x - min_x, max_y - min_y).unwrap()
    }

    fn chop_harf(&self) -> (Line, Line) {
        self.chop(0.5)
    }

    fn chop(&self, position: f32) -> (Line, Line) {
        let new_x = self.from.x + position * (self.to.x - self.from.x);
        let new_y = self.from.y + position * (self.to.y - self.from.y);
        let mid_point = Point::from_xy(new_x, new_y);
        (
            Line {
                from: self.from,
                to: mid_point,
            },
            Line {
                from: mid_point,
                to: self.to,
            },
        )
    }

    fn to_path_segment(self) -> PathSegment {
        PathSegment::Line(self)
    }

    fn reverse(&self) -> Self {
        Line {
            from: self.to,
            to: self.from,
        }
    }

    fn is_same_or_reversed(&self, other: &Self) -> bool {
        self == other || self == &other.reverse()
    }

    fn from_vector(&self) -> Point {
        self.to - self.from
    }

    fn to_vector(&self) -> Point {
        self.to - self.from
    }

    fn polygon(&self) -> Vec<Point> {
        vec![self.from, self.to]
    }
}

#[derive(Debug, PartialEq, Clone)]

pub(crate) struct Quadratic {
    pub(crate) from: Point,
    pub(crate) to: Point,
    pub(crate) control: Point,
}

impl SegmentTrait for Quadratic {
    fn move_to(&self, point: Point) -> Self {
        Quadratic {
            from: self.from + point,
            to: self.to + point,
            control: self.control + point,
        }
    }

    fn set_from(&mut self, point: Point) {
        self.from = point;
    }

    fn set_to(&mut self, point: Point) {
        self.to = point;
    }

    fn endpoints(&self) -> (Point, Point) {
        (self.from, self.to)
    }

    fn rect(&self) -> Rect {
        let min_x = self.from.x.min(self.to.x).min(self.control.x);
        let min_y = self.from.y.min(self.to.y).min(self.control.y);
        let max_x = self.from.x.max(self.to.x).max(self.control.x);
        let max_y = self.from.y.max(self.to.y).max(self.control.y);
        Rect::from_xywh(min_x, min_y, max_x - min_x, max_y - min_y).unwrap()
    }

    fn chop_harf(&self) -> (Quadratic, Quadratic) {
        self.chop(0.5)
    }

    fn chop(&self, position: f32) -> (Quadratic, Quadratic) {
        let mut result = [Point::default(); 5];
        let center = NormalizedF32Exclusive::new_bounded(position);
        let arg = [self.from, self.control, self.to];
        path_geometry::chop_quad_at(&arg, center, &mut result);
        (
            Quadratic {
                from: result[0],
                to: result[2],
                control: result[1],
            },
            Quadratic {
                from: result[2],
                to: result[4],
                control: result[3],
            },
        )
    }

    fn to_path_segment(self) -> PathSegment {
        PathSegment::Quadratic(self)
    }

    fn reverse(&self) -> Self {
        Quadratic {
            from: self.to,
            to: self.from,
            control: self.control,
        }
    }

    fn is_same_or_reversed(&self, other: &Self) -> bool {
        self == other || self == &other.reverse()
    }

    fn from_vector(&self) -> Point {
        self.control - self.from
    }

    fn to_vector(&self) -> Point {
        self.to - self.control
    }

    fn polygon(&self) -> Vec<Point> {
        vec![self.from, self.control, self.to]
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Cubic {
    pub(crate) from: Point,
    pub(crate) to: Point,
    pub(crate) control1: Point,
    pub(crate) control2: Point,
}

impl SegmentTrait for Cubic {
    fn move_to(&self, point: Point) -> Self {
        Cubic {
            from: self.from + point,
            to: self.to + point,
            control1: self.control1 + point,
            control2: self.control2 + point,
        }
    }

    fn set_from(&mut self, point: Point) {
        self.from = point;
    }

    fn set_to(&mut self, point: Point) {
        self.to = point;
    }

    fn endpoints(&self) -> (Point, Point) {
        (self.from, self.to)
    }

    fn rect(&self) -> Rect {
        let min_x = self
            .from
            .x
            .min(self.to.x)
            .min(self.control1.x)
            .min(self.control2.x);
        let min_y = self
            .from
            .y
            .min(self.to.y)
            .min(self.control1.y)
            .min(self.control2.y);
        let max_x = self
            .from
            .x
            .max(self.to.x)
            .max(self.control1.x)
            .max(self.control2.x);
        let max_y = self
            .from
            .y
            .max(self.to.y)
            .max(self.control1.y)
            .max(self.control2.y);
        Rect::from_xywh(min_x, min_y, max_x - min_x, max_y - min_y).unwrap()
    }

    fn chop_harf(&self) -> (Cubic, Cubic) {
        self.chop(0.5)
    }

    fn chop(&self, position: f32) -> (Cubic, Cubic) {
        let mut result = [Point::default(); 7];
        let center = NormalizedF32Exclusive::new_bounded(position);
        let arg = [self.from, self.control1, self.control2, self.to];
        path_geometry::chop_cubic_at2(&arg, center, &mut result);
        (
            Cubic {
                from: result[0],
                to: result[3],
                control1: result[1],
                control2: result[2],
            },
            Cubic {
                from: result[3],
                to: result[6],
                control1: result[4],
                control2: result[5],
            },
        )
    }

    fn to_path_segment(self) -> PathSegment {
        PathSegment::Cubic(self)
    }

    fn reverse(&self) -> Self {
        Cubic {
            from: self.to,
            to: self.from,
            control1: self.control2,
            control2: self.control1,
        }
    }

    fn is_same_or_reversed(&self, other: &Self) -> bool {
        self == other || self == &other.reverse()
    }

    fn from_vector(&self) -> Point {
        self.control1 - self.from
    }

    fn to_vector(&self) -> Point {
        self.to - self.control2
    }

    fn polygon(&self) -> Vec<Point> {
        let mut points = vec![self.from, self.control1, self.control2, self.to];
        let center = points.iter().fold(Point::zero(), |sum, p| Point {
            x: sum.x + p.x,
            y: sum.y + p.y,
        });
        let center = Point {
            x: center.x / 4.0,
            y: center.y / 4.0,
        };

        points.sort_by(|l, r| cmp_clockwise(&center, l, r));
        return [points[0], points[1], points[2], points[3]].to_vec();
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PathSegment {
    Line(Line),
    Quadratic(Quadratic),
    Cubic(Cubic),
}

impl PathSegment {
    #[allow(dead_code)]
    pub(crate) fn move_to(&self, point: Point) -> Self {
        match self {
            PathSegment::Line(line) => PathSegment::Line(line.move_to(point)),
            PathSegment::Quadratic(quad) => PathSegment::Quadratic(quad.move_to(point)),
            PathSegment::Cubic(cubic) => PathSegment::Cubic(cubic.move_to(point)),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn set_from(&mut self, point: Point) {
        match self {
            PathSegment::Line(line) => line.set_from(point),
            PathSegment::Quadratic(quad) => quad.set_from(point),
            PathSegment::Cubic(cubic) => cubic.set_from(point),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn set_to(&mut self, point: Point) {
        match self {
            PathSegment::Line(line) => line.set_to(point),
            PathSegment::Quadratic(quad) => quad.set_to(point),
            PathSegment::Cubic(cubic) => cubic.set_to(point),
        }
    }

    pub(crate) fn endpoints(&self) -> (Point, Point) {
        match self {
            PathSegment::Line(line) => line.endpoints(),
            PathSegment::Quadratic(quad) => quad.endpoints(),
            PathSegment::Cubic(cubic) => cubic.endpoints(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn rect(&self) -> Rect {
        match self {
            PathSegment::Line(line) => line.rect(),
            PathSegment::Quadratic(quad) => quad.rect(),
            PathSegment::Cubic(cubic) => cubic.rect(),
        }
    }

    /// position で指定された位置でセグメントを分割する
    /// position は 0.0 から 1.0 の範囲で指定する
    pub(crate) fn chop(&self, position: f32) -> (PathSegment, PathSegment) {
        match self {
            PathSegment::Line(line) => {
                let (line1, line2) = line.chop(position);
                (PathSegment::Line(line1), PathSegment::Line(line2))
            }
            PathSegment::Quadratic(quad) => {
                let (quad1, quad2) = quad.chop(position);
                (PathSegment::Quadratic(quad1), PathSegment::Quadratic(quad2))
            }
            PathSegment::Cubic(cubic) => {
                let (cubic1, cubic2) = cubic.chop(position);
                (PathSegment::Cubic(cubic1), PathSegment::Cubic(cubic2))
            }
        }
    }

    pub(crate) fn reverse(&self) -> Self {
        match self {
            PathSegment::Line(line) => PathSegment::Line(line.reverse()),
            PathSegment::Quadratic(quad) => PathSegment::Quadratic(quad.reverse()),
            PathSegment::Cubic(cubic) => PathSegment::Cubic(cubic.reverse()),
        }
    }

    pub(crate) fn is_same_or_reversed(&self, other: &Self) -> bool {
        match self {
            PathSegment::Line(line) => line.is_same_or_reversed(match other {
                PathSegment::Line(line) => line,
                _ => return false,
            }),
            PathSegment::Quadratic(quad) => quad.is_same_or_reversed(match other {
                PathSegment::Quadratic(quad) => quad,
                _ => return false,
            }),
            PathSegment::Cubic(cubic) => cubic.is_same_or_reversed(match other {
                PathSegment::Cubic(cubic) => cubic,
                _ => return false,
            }),
        }
    }

    #[allow(clippy::wrong_self_convention, dead_code)]
    pub(crate) fn from_vector(&self) -> Point {
        match self {
            PathSegment::Line(line) => line.from_vector(),
            PathSegment::Quadratic(quad) => quad.from_vector(),
            PathSegment::Cubic(cubic) => cubic.from_vector(),
        }
    }

    pub(crate) fn to_vector(&self) -> Point {
        match self {
            PathSegment::Line(line) => line.to_vector(),
            PathSegment::Quadratic(quad) => quad.to_vector(),
            PathSegment::Cubic(cubic) => cubic.to_vector(),
        }
    }

    #[inline]
    #[allow(clippy::wrong_self_convention)]
    fn from_vector_candidates(&self) -> [Point; 3] {
        match self {
            PathSegment::Line(line) => [line.from_vector(), line.from_vector(), line.from_vector()],
            PathSegment::Quadratic(quad) => {
                [quad.from_vector(), quad.to_vector(), quad.to_vector()]
            }
            PathSegment::Cubic(cubic) => [
                cubic.from_vector(),
                cubic.control2 - cubic.control1,
                cubic.to_vector(),
            ],
        }
    }

    #[inline]
    fn cmp_clockwise_vector(base: &PathSegment, l: &PathSegment, r: &PathSegment) -> Ordering {
        let base_vector = -base.to_vector();
        let l_vectors = l.from_vector_candidates();
        let r_vectors = r.from_vector_candidates();
        (0..3)
            .map(|i| cmp_clockwise(&base_vector, &l_vectors[i], &r_vectors[i]))
            .find(|o| *o != Ordering::Equal)
            .unwrap_or(Ordering::Equal)
    }

    #[inline]
    pub(crate) fn select_clockwise_vector(&self, segments: &[PathSegment]) -> PathSegment {
        segments
            .iter()
            .max_by(|l, r| Self::cmp_clockwise_vector(self, l, r))
            .unwrap()
            .clone()
    }

    #[inline]
    pub(crate) fn select_counter_clockwise_vector(&self, segments: &[PathSegment]) -> PathSegment {
        segments
            .iter()
            .min_by(|l, r| Self::cmp_clockwise_vector(self, l, r))
            .unwrap()
            .clone()
    }

    #[inline]
    pub(crate) fn polygon(&self) -> Vec<Point> {
        match self {
            PathSegment::Line(line) => line.polygon(),
            PathSegment::Quadratic(quad) => quad.polygon(),
            PathSegment::Cubic(cubic) => cubic.polygon(),
        }
    }
}

#[cfg(test)]
mod tests {
    use tiny_skia_path::Point;

    use crate::{Line, PathSegment, Quadratic};

    #[test]
    fn test_select_clockwise_vector() {
        let p0 = Point::from_xy(0.0, 100.0);
        let p1 = Point::from_xy(100.0, 100.0);
        let p2 = Point::from_xy(200.0, 100.0);
        let p3 = Point::from_xy(200.0, 200.0);

        let segment1 = PathSegment::Line(Line { from: p0, to: p1 });
        let segment2 = PathSegment::Line(Line { from: p1, to: p2 });
        let segment3 = PathSegment::Quadratic(Quadratic {
            from: p1,
            to: p3,
            control: p2,
        });
        assert_eq!(
            segment1.select_clockwise_vector(&[segment2.clone(), segment3.clone()]),
            segment2.clone()
        );
        assert_eq!(
            segment1.select_clockwise_vector(&[segment3.clone(), segment2.clone()]),
            segment2.clone()
        );
    }
}
