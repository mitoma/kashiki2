use kashiki2::smooth_value::*;
use piston::input;
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

    let mut frame: u16 = 0;
    let mut x_smooth: SmoothValue = SmoothValue::new(10.0, MovingType::Liner, 5);

    while let Some(event) = window.next() {
        if let Some(text) = event.text_args() {
            input_text = text;
        }
        if let Some(key) = event.press_args() {
            match key {
                input::Button::Keyboard(keyboard::Key::Right) => {
                    x_smooth.add(100.0);
                }
                input::Button::Keyboard(keyboard::Key::Left) => {
                    x_smooth.add(-100.0);
                }
                _ => {}
            }
        }
        if let Some(_args) = event.render_args() {
            frame = frame + 1;
            window.draw_2d(&event, |context, graphics, device| {
                // Set a white background
                let transform = context.transform.trans(10.0 + x_smooth.next(), 100.0);
                clear([1.0, 1.0, 1.0, 1.0], graphics);
                /*
                text::Text::new_color([0.0, 0.0, 0.0, 1.0], 64)
                    .draw(
                        &format!("{}", frame),
                        &mut glyphs,
                        &context.draw_state,
                        transform,
                        graphics,
                    )
                    .unwrap();
                    */
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
