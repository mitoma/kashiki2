use gui::key_adapter::*;
use gui::smooth_value::*;
use piston_window::*;

fn main() {
    let mut window: PistonWindow = WindowSettings::new("Kashiki", [640, 480])
        .exit_on_esc(true)
        .build()
        .unwrap();

    let caret = &mut text_buffer::caret::Caret::new(0, 0);
    let mut text_buffer = text_buffer::buffer::Buffer::new();
    text_buffer.insert_string(caret, "hello world".to_string());

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

        if let Some(action) = input_action {
            println!("event:{:?}, input:{:?}", event, action);
            match action {
                InputAction::TextAction(s) => text_buffer.insert_string(caret, s),
                InputAction::KeyAction(key_with_meta) => {
                    match key_with_meta {
                        KeyWithMeta {
                            key: gui::key_adapter::Key::Backspace,
                            meta_key: MetaKey::None,
                        } => {
                            text_buffer.backspace(caret);
                        }
                        KeyWithMeta {
                            key: gui::key_adapter::Key::Return,
                            meta_key: MetaKey::None,
                        } => {
                            text_buffer.insert_enter(caret);
                        }

                        KeyWithMeta {
                            key: gui::key_adapter::Key::Left,
                            meta_key: MetaKey::None,
                        } => {
                            text_buffer.back(caret);
                        }
                        KeyWithMeta {
                            key: gui::key_adapter::Key::Right,
                            meta_key: MetaKey::None,
                        } => {
                            println!("go foward!");
                            text_buffer.forward(caret);
                        }
                        KeyWithMeta {
                            key: gui::key_adapter::Key::Up,
                            meta_key: MetaKey::None,
                        } => {
                            text_buffer.previous(caret);
                        }
                        KeyWithMeta {
                            key: gui::key_adapter::Key::Down,
                            meta_key: MetaKey::None,
                        } => {
                            text_buffer.next(caret);
                        }

                        //
                        KeyWithMeta {
                            key: gui::key_adapter::Key::Left,
                            meta_key: MetaKey::Shift,
                        } => {
                            x_smooth.add(-100.0);
                        }
                        KeyWithMeta {
                            key: gui::key_adapter::Key::Right,
                            meta_key: MetaKey::Shift,
                        } => {
                            x_smooth.add(100.0);
                        }
                        KeyWithMeta {
                            key: gui::key_adapter::Key::Up,
                            meta_key: MetaKey::Shift,
                        } => {
                            y_smooth.add(-100.0);
                        }
                        KeyWithMeta {
                            key: gui::key_adapter::Key::Down,
                            meta_key: MetaKey::Shift,
                        } => {
                            y_smooth.add(100.0);
                        }
                        _ => {}
                    };
                }
            }
        }

        window.draw_2d(&event, |context, graphics, device| {
            // Set a white background
            clear([1.0, 1.0, 1.0, 1.0], graphics);

            let base_x = x_smooth.next();
            let base_y = y_smooth.next();

            let mut gain_x = 0.0;
            let mut gain_y = 0.0;

            let transform = context.transform.trans(base_x + gain_x, base_y + gain_y);

            text_buffer.lines.iter().for_each(|line| {
                gain_y += 18.0;
                line.chars.iter().for_each(|c| {
                    gain_x += 18.0;

                    let transform = transform.trans(base_x + gain_x, base_y + gain_y);

                    text::Text::new_color([0.0, 0.0, 0.0, 1.0], 16)
                        .draw(
                            &c.c.to_string(),
                            &mut glyphs,
                            &context.draw_state,
                            transform,
                            graphics,
                        )
                        .unwrap();
                });
                gain_x = 0.0;
            });

            let transform = transform.trans(
                base_x + ((caret.col + 1) as f64 * 18.0),
                base_y + ((caret.row + 1) as f64 * 18.0),
            );
            text::Text::new_color([0.5, 0.5, 0.0, 1.0], 16)
                .draw("◆", &mut glyphs, &context.draw_state, transform, graphics)
                .unwrap();
            // Update glyphs before rendering.
            glyphs.factory.encoder.flush(device);
        });
    }
}
