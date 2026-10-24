use bezier_converter::CubicBezier;
use skrifa::outline::OutlinePen;

use crate::straight_run_simplifier::is_nearly_straight_default;

#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct QuadraticCurve {
    pub start: [f32; 2],
    pub control: [f32; 2],
    pub end: [f32; 2],
}

impl QuadraticCurve {
    pub(crate) fn points(self) -> [[f32; 2]; 3] {
        [self.start, self.control, self.end]
    }
}

#[derive(Debug)]
pub struct VectorVertex {
    pub(crate) curves: Vec<QuadraticCurve>,
}

impl VectorVertex {
    pub fn curves(&self) -> &[QuadraticCurve] {
        &self.curves
    }
}

#[derive(Default)]
pub struct VectorVertexBuilder {
    curves: Vec<QuadraticCurve>,
    current: Option<[f32; 2]>,
    subpath_start: Option<[f32; 2]>,
    builder_options: VertexBuilderOptions,
}

impl VectorVertexBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn with_options(mut self, builder_options: VertexBuilderOptions) -> Self {
        self.builder_options = builder_options;
        self
    }

    pub fn build(self) -> VectorVertex {
        let options = self.builder_options;
        let center = options
            .coordinate_system
            .transform(options.center[0], options.center[1]);
        let center = options.scale.map_or(center, |scale| {
            std::array::from_fn(|axis| center[axis] * scale[axis])
        });
        let transform = |point: [f32; 2]| {
            let point = options.coordinate_system.transform(point[0], point[1]);
            let point = std::array::from_fn(|axis| (point[axis] - center[axis]) / options.unit_em);
            options.scale.map_or(point, |scale| {
                std::array::from_fn(|axis| point[axis] * scale[axis])
            })
        };
        VectorVertex {
            curves: self
                .curves
                .into_iter()
                .map(|curve| QuadraticCurve {
                    start: transform(curve.start),
                    control: transform(curve.control),
                    end: transform(curve.end),
                })
                .collect(),
        }
    }

    pub fn move_to(&mut self, x: f32, y: f32) {
        self.current = Some([x, y]);
        self.subpath_start = self.current;
    }

    pub fn line_to(&mut self, x: f32, y: f32) {
        let Some(start) = self.current else {
            return;
        };
        let end = [x, y];
        if start == end {
            return;
        }
        self.curves.push(QuadraticCurve {
            start,
            control: std::array::from_fn(|axis| (start[axis] + end[axis]) * 0.5),
            end,
        });
        self.current = Some(end);
    }

    pub fn quad_to(&mut self, control_x: f32, control_y: f32, x: f32, y: f32) {
        let Some(start) = self.current else {
            return;
        };
        let control = [control_x, control_y];
        let end = [x, y];
        if start == control && start == end {
            return;
        }
        if is_nearly_straight_default(start.into(), control.into(), end.into()) {
            self.line_to(x, y);
            return;
        }
        self.curves.push(QuadraticCurve {
            start,
            control,
            end,
        });
        self.current = Some(end);
    }

    pub fn curve_to(
        &mut self,
        control_x1: f32,
        control_y1: f32,
        control_x2: f32,
        control_y2: f32,
        x: f32,
        y: f32,
    ) {
        let Some(start) = self.current else {
            return;
        };
        if start == [control_x1, control_y1] && start == [control_x2, control_y2] && start == [x, y]
        {
            return;
        }
        let cubic = CubicBezier {
            x0: start[0],
            y0: start[1],
            x1: x,
            y1: y,
            cx0: control_x1,
            cy0: control_y1,
            cx1: control_x2,
            cy1: control_y2,
        };
        for curve in cubic.to_quadratic() {
            self.quad_to(curve.cx0, curve.cy0, curve.x1, curve.y1);
        }
    }

    pub fn close(&mut self) {
        if let Some(start) = self.subpath_start.take() {
            self.line_to(start[0], start[1]);
        }
    }
}

impl OutlinePen for VectorVertexBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.move_to(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.line_to(x, y);
    }
    fn quad_to(&mut self, control_x: f32, control_y: f32, x: f32, y: f32) {
        self.quad_to(control_x, control_y, x, y);
    }
    fn curve_to(
        &mut self,
        control_x1: f32,
        control_y1: f32,
        control_x2: f32,
        control_y2: f32,
        x: f32,
        y: f32,
    ) {
        self.curve_to(control_x1, control_y1, control_x2, control_y2, x, y);
    }
    fn close(&mut self) {
        self.close();
    }
}

pub enum CoordinateSystem {
    Svg,
    Font,
}

impl CoordinateSystem {
    pub(crate) fn transform(&self, x: f32, y: f32) -> [f32; 2] {
        match self {
            Self::Svg => [x, -y],
            Self::Font => [x, y],
        }
    }
}

pub(crate) struct VertexBuilderOptions {
    pub(crate) center: [f32; 2],
    pub(crate) unit_em: f32,
    pub(crate) coordinate_system: CoordinateSystem,
    pub(crate) scale: Option<[f32; 2]>,
}

impl Default for VertexBuilderOptions {
    fn default() -> Self {
        Self {
            center: [0.0; 2],
            unit_em: 1.0,
            coordinate_system: CoordinateSystem::Font,
            scale: None,
        }
    }
}

impl VertexBuilderOptions {
    pub fn new(
        center: [f32; 2],
        unit_em: f32,
        coordinate_system: CoordinateSystem,
        scale: Option<[f32; 2]>,
    ) -> Self {
        Self {
            center,
            unit_em,
            coordinate_system,
            scale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_directed_curves_without_auxiliary_geometry() {
        let mut builder = VectorVertexBuilder::new();
        builder.move_to(0.0, 0.0);
        builder.line_to(4.0, 0.0);
        builder.quad_to(6.0, 3.0, 4.0, 4.0);
        builder.close();
        let path = builder.build();
        assert_eq!(
            path.curves(),
            &[
                QuadraticCurve {
                    start: [0.0, 0.0],
                    control: [2.0, 0.0],
                    end: [4.0, 0.0]
                },
                QuadraticCurve {
                    start: [4.0, 0.0],
                    control: [6.0, 3.0],
                    end: [4.0, 4.0]
                },
                QuadraticCurve {
                    start: [4.0, 4.0],
                    control: [2.0, 2.0],
                    end: [0.0, 0.0]
                },
            ]
        );
    }

    #[test]
    fn separate_contours_preserve_direction_without_connecting_edges() {
        let mut builder = VectorVertexBuilder::new();
        for points in [
            [[0.0, 0.0], [4.0, 0.0], [4.0, 4.0]],
            [[1.0, 1.0], [1.0, 2.0], [2.0, 1.0]],
        ] {
            builder.move_to(points[0][0], points[0][1]);
            for point in &points[1..] {
                builder.line_to(point[0], point[1]);
            }
            builder.close();
            builder.close();
        }
        let path = builder.build();
        assert_eq!(path.curves().len(), 6);
        assert_eq!(path.curves()[2].end, [0.0, 0.0]);
        assert_eq!(path.curves()[3].start, [1.0, 1.0]);
        assert_eq!(path.curves()[5].end, [1.0, 1.0]);
    }

    #[test]
    fn transforms_all_curve_points() {
        let mut builder = VectorVertexBuilder::new().with_options(VertexBuilderOptions::new(
            [2.0, 4.0],
            2.0,
            CoordinateSystem::Svg,
            None,
        ));
        builder.move_to(2.0, 4.0);
        builder.quad_to(4.0, 2.0, 6.0, 4.0);
        assert_eq!(
            builder.build().curves()[0],
            QuadraticCurve {
                start: [0.0, 0.0],
                control: [1.0, 1.0],
                end: [2.0, 0.0]
            }
        );

        let mut builder = VectorVertexBuilder::new().with_options(VertexBuilderOptions::new(
            [0.0; 2],
            2.0,
            CoordinateSystem::Font,
            Some([2.0, 3.0]),
        ));
        builder.move_to(2.0, 4.0);
        builder.quad_to(4.0, 2.0, 6.0, 4.0);
        assert_eq!(
            builder.build().curves()[0],
            QuadraticCurve {
                start: [2.0, 6.0],
                control: [4.0, 3.0],
                end: [6.0, 6.0]
            }
        );
    }

    #[test]
    fn cubic_approximation_preserves_endpoints_and_continuity() {
        let mut builder = VectorVertexBuilder::new();
        builder.move_to(0.0, 0.0);
        builder.curve_to(0.0, 4.0, 4.0, 4.0, 4.0, 0.0);
        builder.close();
        let path = builder.build();
        assert!(path.curves().len() >= 2);
        assert_eq!(path.curves()[0].start, [0.0, 0.0]);
        assert_eq!(path.curves()[path.curves().len() - 2].end, [4.0, 0.0]);
        assert_eq!(path.curves().last().unwrap().end, [0.0, 0.0]);
        for adjacent in path.curves().windows(2) {
            assert_eq!(adjacent[0].end, adjacent[1].start);
        }
    }

    #[test]
    fn degenerate_commands_do_not_create_segments() {
        let mut builder = VectorVertexBuilder::new();
        builder.line_to(1.0, 1.0);
        builder.quad_to(0.0, 1.0, 1.0, 1.0);
        builder.curve_to(0.0, 1.0, 1.0, 0.0, 1.0, 1.0);
        builder.close();
        builder.move_to(1.0, 1.0);
        builder.line_to(1.0, 1.0);
        builder.quad_to(1.0, 1.0, 1.0, 1.0);
        builder.close();
        assert!(builder.build().curves().is_empty());
    }
}
