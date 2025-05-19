use std::collections::BTreeMap;

use usvg::tiny_skia_path::{PathSegment, Point};

use crate::{
    errors::FontRasterizerError,
    vector_instances::{InstanceAttributes, VectorInstances},
    vector_vertex::{CoordinateSystem, VectorVertex, VectorVertexBuilder, VertexBuilderOptions},
    vector_vertex_buffer::VectorVertexBuffer,
};

pub struct SvgVertexBuffer {
    vertex_buffer: VectorVertexBuffer<String>,
}

impl Default for SvgVertexBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl SvgVertexBuffer {
    fn new() -> Self {
        Self {
            vertex_buffer: VectorVertexBuffer::new(),
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

    pub fn vector_vertex_buffer(&self) -> &VectorVertexBuffer<String> {
        &self.vertex_buffer
    }
}

pub struct SvgBuffers {
    vertex_buffer: VectorVertexBuffer<String>,
    instances: BTreeMap<String, VectorInstances<String>>,
}

impl Default for SvgBuffers {
    fn default() -> Self {
        Self::new()
    }
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

    pub fn vector_vertex_buffer(&self) -> &VectorVertexBuffer<String> {
        &self.vertex_buffer
    }

    pub fn vector_instances(&self) -> Vec<&VectorInstances<String>> {
        self.instances.values().collect()
    }
}

#[allow(non_upper_case_globals)]
pub fn svg_to_vector_vertex(svg: &str) -> Result<VectorVertex, FontRasterizerError> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|err| {
        log::error!("svg parse error: {:?}", err);
        FontRasterizerError::SvgParseError
    })?;

    let mut paths: Vec<_> = vec![];

    for node in tree.root().children() {
        let transform = node.abs_transform();
        match node {
            usvg::Node::Group(group) => group.children().iter().for_each(|child| {
                if let usvg::Node::Path(path) = child {
                    paths.push((path, transform));
                }
            }),
            usvg::Node::Path(path) => paths.push((path, transform)),
            usvg::Node::Image(_image) => todo!(),
            usvg::Node::Text(_text) => todo!(),
        }
    }

    let rect = tree.root().bounding_box();
    let center = [rect.width() / 2.0, rect.height() / 2.0];
    let ratio = if rect.width() > rect.height() {
        rect.width()
    } else {
        rect.height()
    };
    let unit_em = ratio;
    let mut builder = VectorVertexBuilder::new().with_options(VertexBuilderOptions::new(
        center,
        unit_em,
        CoordinateSystem::Svg,
        None,
    ));

    for (path, transform) in paths {
        let mut start_to: Option<Point> = None;
        for segment in path.data().segments() {
            log::info!("{:?}", segment);
            match segment {
                PathSegment::MoveTo(mut point) => {
                    transform.map_point(&mut point);
                    builder.move_to(point.x, point.y);
                    start_to = Some(point);
                }
                PathSegment::LineTo(mut point) => {
                    transform.map_point(&mut point);
                    builder.line_to(point.x, point.y);
                }
                PathSegment::QuadTo(mut point1, mut point) => {
                    transform.map_point(&mut point1);
                    transform.map_point(&mut point);
                    builder.quad_to(point1.x, point1.y, point.x, point.y);
                }
                PathSegment::CubicTo(mut point1, mut point2, mut point) => {
                    transform.map_point(&mut point1);
                    transform.map_point(&mut point2);
                    transform.map_point(&mut point);
                    builder.curve_to(point1.x, point1.y, point2.x, point2.y, point.x, point.y);
                }
                PathSegment::Close => {
                    if let Some(start_to) = start_to {
                        // start_to は事前に map_point されているので、transform は不要
                        builder.line_to(start_to.x, start_to.y);
                    }
                }
            }
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
            <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" >
                <path d="M 10 10 L 90 10 L 90 90 L 10 90 Z" />
                <polygon points="20,20 80,20 80,80 20,80" />
            </svg>
        "#;
        let result = svg_to_vector_vertex(svg);
        assert!(result.is_ok());
        println!("{:?}", result);
    }

    #[test]
    fn test_svg_buffers3() {
        env_logger::builder().is_test(true).try_init().ok();
        let svg = include_str!("../data/sample.svg");
        let result = svg_to_vector_vertex(svg);
        println!("{:?}", result);
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
