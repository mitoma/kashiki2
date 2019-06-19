extern crate piston_window;

use piston_window::*;

fn main() {
    let mut window: PistonWindow = WindowSettings::new("Kashiki", [640, 480])
        .exit_on_esc(true)
        .build()
        .unwrap();

    let mut input_text = "☆".to_string();

    let mut glyphs = Glyphs::new(
        "asset/TakaoGothic.ttf",
        TextureContext {
            factory: window.factory.clone(),
            encoder: window.factory.create_command_buffer().into(),
        },
        TextureSettings::new(),
    )
    .unwrap();

    let mut x: f64 = 0.0;
    let mut frame: u16 = 0;

    while let Some(event) = window.next() {
        if let Some(text) = event.text_args() {
            input_text = text;
        }
        if let Some(_key) = event.press_args() {
            x = x + 10.0;
        }
        if let Some(_args) = event.render_args() {
            frame = frame + 1;
            window.draw_2d(&event, |context, graphics, device| {
                // Set a white background
                let transform = context.transform.trans(10.0 + x, 100.0);
                clear([1.0, 1.0, 1.0, 1.0], graphics);
                text::Text::new_color([0.0, 0.0, 0.0, 1.0], 64)
                    .draw(
                        &format!("{}", frame),
                        &mut glyphs,
                        &context.draw_state,
                        transform,
                        graphics,
                    )
                    .unwrap();
                text::Text::new_color([0.0, 0.0, 0.0, 1.0], 64)
                    .draw(
                        &input_text,
                        &mut glyphs,
                        &context.draw_state,
                        transform,
                        graphics,
                    )
                    .unwrap();
                // Update glyphs before rendering.
                glyphs.factory.encoder.flush(device);
            });
        }
    }
}
