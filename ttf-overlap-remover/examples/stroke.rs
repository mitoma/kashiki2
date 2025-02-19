use tiny_skia::{Paint, PathBuilder, PathStroker, Pixmap, Stroke, Transform};

fn main() {
    let path = {
        let mut pb = PathBuilder::new();

        // Draw the letter "e"
        pb.move_to(50.0, 100.0);
        pb.line_to(150.0, 100.0);
        pb.cubic_to(150.0, 50.0, 50.0, 50.0, 50.0, 100.0);
        pb.cubic_to(50.0, 150.0, 150.0, 150.0, 150.0, 130.0);
        /*
         */

        pb.finish().unwrap()
    };

    let stroke = Stroke::default();
    let mut pixmap = Pixmap::new(500, 500).unwrap();

    let mut paint = Paint::default();
    paint.set_color_rgba8(0, 127, 0, 255);
    paint.anti_alias = false;

    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    pixmap.save_png("image.png").unwrap();
}
