use std::f32;

use rustybuzz::ttf_parser::OutlineBuilder;
use tiny_skia_path::{Path, PathBuilder, Point};

use crate::{
    path_segment::{Cubic, Line, PathSegment, Quadratic},
    remove_overlap,
};

#[derive(Debug)]
pub struct OverlapRemoveOutlineBuilder {
    builder: Option<PathBuilder>,
    paths: Vec<Path>,
}

impl Default for OverlapRemoveOutlineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlapRemoveOutlineBuilder {
    fn new() -> Self {
        Self {
            builder: Some(PathBuilder::new()),
            paths: Vec::new(),
        }
    }

    pub fn paths(self) -> Vec<Path> {
        self.paths
    }

    pub fn outline<T>(&self, builder: &mut T)
    where
        T: OutlineBuilder,
    {
        let removed_paths = remove_overlap(self.paths.clone());

        removed_paths.iter().for_each(|path| {
            let segments_len = path.segments.len();
            path.segments.iter().enumerate().for_each(|(i, segment)| {
                if i == 0 {
                    let Point { x, y } = segment.endpoints().0;
                    builder.move_to(x, y);
                }

                match segment {
                    PathSegment::Line(Line { to, .. }) => {
                        builder.line_to(to.x, to.y);
                    }
                    PathSegment::Quadratic(Quadratic { control, to, .. }) => {
                        builder.quad_to(control.x, control.y, to.x, to.y);
                    }
                    PathSegment::Cubic(Cubic {
                        control1,
                        control2,
                        to,
                        ..
                    }) => {
                        builder
                            .curve_to(control1.x, control1.y, control2.x, control2.y, to.x, to.y);
                    }
                }

                if i == segments_len - 1 {
                    builder.close();
                }
            });
        });
    }
}

impl OutlineBuilder for OverlapRemoveOutlineBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder.as_mut().unwrap().move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.as_mut().unwrap().line_to(x, y);
    }

    fn quad_to(&mut self, x: f32, y: f32, x1: f32, y1: f32) {
        self.builder.as_mut().unwrap().quad_to(x1, y1, x, y);
    }

    fn curve_to(&mut self, x: f32, y: f32, x1: f32, y1: f32, x2: f32, y2: f32) {
        self.builder
            .as_mut()
            .unwrap()
            .cubic_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        let mut builder = self.builder.replace(PathBuilder::new()).unwrap();
        builder.close();
        self.paths.push(builder.finish().unwrap());
    }
}
