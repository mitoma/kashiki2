use std::collections::BTreeMap;

use log::debug;
use svg::{
    node::element::{
        path::{Command, Data, Position},
        tag::{Path, Polygon, Type, SVG},
    },
    parser::Event,
};

use crate::{
    errors::FontRasterizerError,
    vector_instances::{InstanceAttributes, VectorInstances},
    vector_vertex::{VectorVertex, VectorVertexBuilder, VertexBuilderOptions},
    vector_vertex_buffer::VectorVertexBuffer,
};

struct SvgBuffers {
    vertex_buffer: VectorVertexBuffer<String>,
    instances: BTreeMap<String, VectorInstances<String>>,
}

impl SvgBuffers {
    pub fn new() -> SvgBuffers {
        SvgBuffers {
            vertex_buffer: VectorVertexBuffer::new(),
            instances: BTreeMap::new(),
        }
    }

    pub fn append_svg(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: &str,
        svg: &str,
    ) -> Result<(), FontRasterizerError> {
        let vector_vertex = svg_to_vector_vertex(svg)?;
        self.vertex_buffer
            .append(device, queue, key.to_string(), vector_vertex)
    }

    pub fn append_instance(
        &mut self,
        device: &wgpu::Device,
        key: &str,
        instance: InstanceAttributes,
    ) {
        self.instances
            .entry(key.to_string())
            .or_insert_with(|| VectorInstances::new(key.to_string(), device))
            .push(instance)
    }
}

#[allow(non_upper_case_globals)]
fn svg_to_vector_vertex(svg: &str) -> Result<VectorVertex, FontRasterizerError> {
    let Ok(parser) = svg::read(svg) else {
        debug!("Failed to parse SVG: {}", svg);
        return Err(FontRasterizerError::SvgParseError);
    };

    let mut builder = VectorVertexBuilder::new();

    for event in parser {
        fn abs_position(
            pos: &Position,
            current_position: (f32, f32),
            next_position: (f32, f32),
        ) -> (f32, f32) {
            match pos {
                Position::Absolute => next_position,
                Position::Relative => (
                    current_position.0 + next_position.0,
                    current_position.1 + next_position.1,
                ),
            }
        }

        fn trim_unit(str: &str) -> &str {
            let units = ["em", "px"];
            for unit in units.iter() {
                if str.ends_with(unit) {
                    return &str[..str.len() - unit.len()];
                }
            }
            str
        }

        match event {
            Event::Tag(SVG, Type::Start, attributes) => {
                let width = attributes
                    .get("width")
                    .ok_or(FontRasterizerError::SvgParseError)?;
                let height = attributes
                    .get("height")
                    .ok_or(FontRasterizerError::SvgParseError)?;
                let width = trim_unit(width)
                    .parse::<f32>()
                    .map_err(|_| FontRasterizerError::SvgParseError)?;
                let height = trim_unit(height)
                    .parse::<f32>()
                    .map_err(|_| FontRasterizerError::SvgParseError)?;
                let ratio = if width > height { width } else { height };
                let unit_em = ratio;
                let center = [width / 2.0, height / 2.0];
                builder = builder.with_options(VertexBuilderOptions::new(center, unit_em));
            }
            Event::Tag(Path, t, attributes) if t != Type::End => {
                let data = attributes
                    .get("d")
                    .ok_or(FontRasterizerError::SvgParseError)?;
                let data = Data::parse(data)?;
                let mut current_position = (0.0, 0.0);
                let mut start_position: Option<(f32, f32)> = None;
                for command in data.iter() {
                    match command {
                        Command::Move(position, parameters) => {
                            let (to_x, to_y) = abs_position(
                                position,
                                current_position,
                                (parameters[0], parameters[1]),
                            );
                            builder.move_to(to_x, to_y);
                            current_position = (to_x, to_y);
                            start_position = Some(current_position);
                        }
                        Command::Line(position, parameters) => {
                            let (to_x, to_y) = abs_position(
                                position,
                                current_position,
                                (parameters[0], parameters[1]),
                            );
                            builder.line_to(to_x, to_y);
                            current_position = (to_x, to_y);
                        }
                        Command::HorizontalLine(position, parameters) => {
                            let (to_x, _) =
                                abs_position(position, current_position, (parameters[0], 0.0));
                            builder.line_to(to_x, current_position.1);
                            current_position = (to_x, current_position.1);
                        }
                        Command::VerticalLine(position, parameters) => {
                            let (_, to_y) =
                                abs_position(position, current_position, (0.0, parameters[0]));
                            builder.line_to(current_position.0, to_y);
                            current_position = (current_position.0, to_y);
                        }
                        Command::QuadraticCurve(position, parameters) => {
                            let (to_x1, to_y1) = abs_position(
                                position,
                                current_position,
                                (parameters[0], parameters[1]),
                            );
                            let (to_x, to_y) = abs_position(
                                position,
                                current_position,
                                (parameters[0], parameters[1]),
                            );
                            builder.quad_to(to_x1, to_y1, to_x, to_y);
                            current_position = (to_x, to_y);
                        }
                        Command::SmoothQuadraticCurve(position, parameters) => {
                            // FIXME: SmoothQuadraticCurve としての処理は行わず、QuadraticCurve として処理する
                            let (to_x1, to_y1) = abs_position(
                                position,
                                current_position,
                                (parameters[0], parameters[1]),
                            );
                            let (to_x, to_y) = abs_position(
                                position,
                                current_position,
                                (parameters[0], parameters[1]),
                            );
                            builder.quad_to(to_x1, to_y1, to_x, to_y);
                            current_position = (to_x, to_y);
                        }
                        Command::CubicCurve(position, parameters) => {
                            let (to_x1, to_y1) = abs_position(
                                position,
                                current_position,
                                (parameters[0], parameters[1]),
                            );
                            let (to_x2, to_y2) = abs_position(
                                position,
                                current_position,
                                (parameters[2], parameters[3]),
                            );
                            let (to_x, to_y) = abs_position(
                                position,
                                current_position,
                                (parameters[4], parameters[5]),
                            );
                            builder.curve_to(to_x1, to_y1, to_x2, to_y2, to_x, to_y);
                            current_position = (to_x, to_y);
                        }
                        Command::SmoothCubicCurve(position, parameters) => {
                            // FIXME: SmoothCubicCurve としての処理は行わず、QuadraticCurve として処理する
                            let (to_x1, to_y1) = abs_position(
                                position,
                                current_position,
                                (parameters[0], parameters[1]),
                            );
                            let (to_x, to_y) = abs_position(
                                position,
                                current_position,
                                (parameters[2], parameters[3]),
                            );
                            builder.quad_to(to_x1, to_y1, to_x, to_y);
                            current_position = (to_x, to_y);
                        }
                        Command::EllipticalArc(position, parameters) => {
                            // FIXME: EllipticalArc は未対応。単に Line として処理する
                            let (to_x, to_y) = abs_position(
                                position,
                                current_position,
                                (parameters[2], parameters[3]),
                            );
                            builder.line_to(to_x, to_y);
                            current_position = (to_x, to_y);
                        }
                        Command::Close => {
                            if let Some(start_position) = start_position {
                                builder.line_to(start_position.0, start_position.1);
                                current_position = start_position;
                            }
                        }
                    }
                }
            }
            Event::Tag(Polygon, t, attributes) if t != Type::End => {
                let points = attributes
                    .get("points")
                    .ok_or(FontRasterizerError::SvgParseError)?;
                let points = points
                    .trim()
                    .split(' ')
                    .map(|point| {
                        let mut point = point.split(',');
                        let x = point
                            .next()
                            .map(str::trim)
                            .ok_or(FontRasterizerError::SvgParseError)?
                            .parse::<f32>()
                            .map_err(|_| FontRasterizerError::SvgParseError)?;
                        let y = point
                            .next()
                            .map(str::trim)
                            .ok_or(FontRasterizerError::SvgParseError)?
                            .parse::<f32>()
                            .map_err(|_| FontRasterizerError::SvgParseError)?;
                        Ok((x, y))
                    })
                    .collect::<Result<Vec<_>, FontRasterizerError>>()?;
                let mut point_iter = points.iter();
                let Some(start_position) = point_iter.next().cloned() else {
                    continue;
                };

                builder.move_to(start_position.0, start_position.1);
                for (x, y) in point_iter {
                    builder.line_to(*x, *y);
                }
                builder.line_to(start_position.0, start_position.1);
            }
            _ => {}
        }
    }
    Ok(builder.build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svg_buffers() {
        env_logger::builder().is_test(true).try_init().ok();
        let svg = r#"
            <svg width="100" height="100">
                <path d="M 10 10 L 90 10 L 90 90 L 10 90 Z" />
                <polygon points="20,20 80,20 80,80 20,80" />
            </svg>
        "#;
        let result = svg_to_vector_vertex(svg);
        assert!(result.is_ok());
        println!("{:?}", result);
    }

    #[test]
    fn test_svg_buffers2() {
        env_logger::builder().is_test(true).try_init().ok();
        let svg = include_str!("../data/rice.svg");
        let result = svg_to_vector_vertex(svg);
        println!("{:?}", result);
        assert!(result.is_ok());
        println!("{:?}", result);
    }
}
