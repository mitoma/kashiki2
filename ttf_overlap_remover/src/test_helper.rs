use std::f32;

use tiny_skia::{Color, Paint, Pixmap, PremultipliedColorU8};
use tiny_skia_path::{Path, PathBuilder, Point, Stroke, Transform};

use crate::{
    is_clockwise,
    path_segment::{Cubic, Line, PathSegment, Quadratic},
    path_to_path_segments,
};

// segments と dots が Pixmap の中に納まるような transform を計算する
fn calc_transform(
    canvas_size: f32,
    segments: &Vec<&PathSegment>,
    dots: &Vec<&Point>,
) -> (Transform, f32) {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for segment in segments {
        match segment {
            PathSegment::Line(Line { from, to }) => {
                min_x = min_x.min(from.x).min(to.x);
                min_y = min_y.min(from.y).min(to.y);
                max_x = max_x.max(from.x).max(to.x);
                max_y = max_y.max(from.y).max(to.y);
            }
            PathSegment::Quadratic(Quadratic { from, to, control }) => {
                min_x = min_x.min(from.x).min(to.x).min(control.x);
                min_y = min_y.min(from.y).min(to.y).min(control.y);
                max_x = max_x.max(from.x).max(to.x).max(control.x);
                max_y = max_y.max(from.y).max(to.y).max(control.y);
            }
            PathSegment::Cubic(Cubic {
                from,
                to,
                control1,
                control2,
            }) => {
                min_x = min_x.min(from.x).min(to.x).min(control1.x).min(control2.x);
                min_y = min_y.min(from.y).min(to.y).min(control1.y).min(control2.y);
                max_x = max_x.max(from.x).max(to.x).max(control1.x).max(control2.x);
                max_y = max_y.max(from.y).max(to.y).max(control1.y).max(control2.y);
            }
        }
    }

    for dot in dots {
        min_x = min_x.min(dot.x);
        min_y = min_y.min(dot.y);
        max_x = max_x.max(dot.x);
        max_y = max_y.max(dot.y);
    }

    let width = max_x - min_x;
    let height = max_y - min_y;
    let scale = canvas_size / width.max(height);

    let translate_x = -min_x * scale;
    let translate_y = -min_y * scale;

    (
        Transform::identity()
            .post_scale(scale, -scale)
            .post_translate(translate_x, canvas_size - translate_y),
        scale,
    )
}

pub(crate) fn gen_even_pixmap(paths: &Vec<Path>) -> Pixmap {
    let canvas_size = 500.0;
    let segments = paths
        .iter()
        .cloned()
        .flat_map(path_to_path_segments)
        .collect::<Vec<_>>();

    let (transform, _) = calc_transform(canvas_size, &segments.iter().collect(), &vec![]);
    let mut paint = Paint::default();
    let mut pixmap = Pixmap::new(canvas_size as u32, canvas_size as u32).unwrap();
    paint.anti_alias = false;
    pixmap.fill(Color::WHITE);

    for path in paths {
        let segments = path_to_path_segments(path.clone());
        if !is_clockwise(&segments) {
            paint.set_color_rgba8(255, 0, 0, 16);
            pixmap.fill_path(path, &paint, tiny_skia::FillRule::EvenOdd, transform, None);
        }
    }
    for path in paths {
        let segments = path_to_path_segments(path.clone());
        if is_clockwise(&segments) {
            paint.set_color_rgba8(0, 255, 0, 16);
            pixmap.fill_path(path, &paint, tiny_skia::FillRule::EvenOdd, transform, None);
        }
    }
    pixmap.pixels_mut().iter_mut().for_each(|p| {
        let red = p.red();
        let green = p.green();
        if red > green {
            *p = PremultipliedColorU8::from_rgba(0, 0, 0, 255).unwrap();
        } else {
            *p = PremultipliedColorU8::from_rgba(255, 255, 255, 255).unwrap();
        }
    });
    pixmap
}

pub(crate) fn path_segments_to_image(segments: Vec<&PathSegment>, dots: Vec<&Point>) {
    path_segments_to_images("default", segments, dots);
}

pub(crate) fn path_segments_to_images(name: &str, segments: Vec<&PathSegment>, dots: Vec<&Point>) {
    let canvas_size = 1000.0;
    let (transform, scale) = calc_transform(canvas_size, &segments, &dots);
    let scale_unit = 1.0 / scale;
    //println!("scale: {}, scale_unit: {}", scale, scale_unit);

    let mut paint = Paint::default();
    let mut pixmap = Pixmap::new(canvas_size as u32, canvas_size as u32).unwrap();
    let stroke = Stroke {
        width: scale_unit,
        ..Default::default()
    };
    paint.anti_alias = true;

    let dot_stroke = Stroke {
        width: scale_unit * 5.0,
        line_cap: tiny_skia::LineCap::Round,
        ..Default::default()
    };

    for segment in segments {
        let (from, to) = segment.endpoints();
        let from_dot = {
            let mut from_dot = PathBuilder::new();
            from_dot.move_to(from.x, from.y);
            from_dot.line_to(from.x + f32::EPSILON, from.y + f32::EPSILON);
            from_dot.finish().unwrap()
        };
        let to_dot = {
            let mut to_dot = PathBuilder::new();
            to_dot.move_to(to.x, to.y);
            to_dot.line_to(to.x + f32::EPSILON, to.y + f32::EPSILON);
            to_dot.finish().unwrap()
        };

        let path = {
            let mut pb = PathBuilder::new();
            match segment {
                PathSegment::Line(Line { from, to }) => {
                    pb.move_to(from.x, from.y);
                    pb.line_to(to.x, to.y);
                }
                PathSegment::Quadratic(Quadratic { from, to, control }) => {
                    pb.move_to(from.x, from.y);
                    pb.quad_to(control.x, control.y, to.x, to.y);
                }
                PathSegment::Cubic(Cubic {
                    from,
                    to,
                    control1,
                    control2,
                }) => {
                    pb.move_to(from.x, from.y);
                    pb.cubic_to(control1.x, control1.y, control2.x, control2.y, to.x, to.y);
                }
            }
            pb.finish().unwrap()
        };

        paint.set_color_rgba8(0, 127, 0, 255);
        pixmap.stroke_path(&path, &paint, &stroke, transform, None);
        paint.set_color_rgba8(255, 0, 0, 120);
        pixmap.stroke_path(&from_dot, &paint, &dot_stroke, transform, None);
        paint.set_color_rgba8(0, 255, 0, 120);
        pixmap.stroke_path(&to_dot, &paint, &dot_stroke, transform, None);
    }

    paint.set_color_rgba8(0, 0, 255, 255);
    for dot in dots {
        let mut dot_path = PathBuilder::new();
        dot_path.move_to(dot.x, dot.y);
        dot_path.line_to(dot.x + f32::EPSILON, dot.y + f32::EPSILON);
        let dot_path = dot_path.finish().unwrap();
        pixmap.stroke_path(&dot_path, &paint, &dot_stroke, transform, None);
    }

    pixmap
        .save_png(format!("image/image_{}.png", name))
        .unwrap();
}
