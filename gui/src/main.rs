use gui::key_adapter::*;
use gui::smooth_value::*;
use piston::input;
use piston_window::*;

fn main() {
    let mut window: PistonWindow = WindowSettings::new("Kashiki", [640, 480])
        .exit_on_esc(true)
        .build()
        .unwrap();

    let mut caret = text_buffer::caret::Caret::new(0, 0);
    let mut text_buffer = text_buffer::buffer::Buffer::new();

    let mut glyphs = Glyphs::new(
        "asset/TakaoGothic.ttf",
        TextureContext {
            factory: window.factory.clone(),
            encoder: window.factory.create_command_buffer().into(),
        },
        TextureSettings::new(),
    )
    .unwrap();

    let mut x_smooth: SmoothValue = SmoothValue::new(10.0, MovingType::Smooth, 10);
    let mut y_smooth: SmoothValue = SmoothValue::new(10.0, MovingType::Smooth, 10);

    let mut adapter = StrokeParser::new();

    while let Some(event) = window.next() {
        let input_action = adapter.parse(&event);

        if let Some(_args) = event.render_args() {
            window.draw_2d(&event, |context, graphics, device| {
                // Set a white background
                let transform = context
                    .transform
                    .trans(10.0 + x_smooth.next(), 100.0 + y_smooth.next());
                clear([1.0, 1.0, 1.0, 1.0], graphics);
                text::Text::new_color([0.0, 0.0, 0.0, 1.0], 64)
                    .draw(
                        &text_buffer.to_buffer_string(),
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
