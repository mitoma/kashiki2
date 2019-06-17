extern crate piston_window;

use piston_window::*;

fn main() {
    let mut window: PistonWindow = WindowSettings::new("Hello Piston!", [640, 480])
        .exit_on_esc(true)
        .build()
        .unwrap();

    let factory = window.factory.clone();
    let mut glyphs = Glyphs::new("asset/TakaoGothic.ttf", factory, TextureSettings::new()).unwrap();

    let mut input_text = "hello".to_string();

    while let Some(event) = window.next() {
        /*
        if let Some(text) = event.text_args() {
            //println!("{}", text);
        }
        if let Some(cursor) = event.cursor_args() {
            //println!("cursor:{}", cursor);
        }
        if let Some(mouse) = event.mouse_cursor_args() {
            //println!("mouse:[{}, {}]", mouse[0], mouse[1])
        }
        */
        if let Some(text) = event.text_args() {
            input_text = text;
        }
        window.draw_2d(&event, |context, graphics| {
            /*
            clear([1.0; 4], graphics);
            rectangle(
                [1.0, 0.0, 0.0, 1.0], // red
                [0.0, 0.0, 100.0, 100.0],
                context.transform,
                graphics,
            );
            */
            // Set a white background
            let transform = context.transform.trans(10.0, 100.0);
            clear([1.0, 1.0, 1.0, 1.0], graphics);
            text::Text::new_color([0.0, 0.0, 0.0, 1.0], 32)
                .draw(
                    &input_text,
                    &mut glyphs,
                    &context.draw_state,
                    transform,
                    graphics,
                )
                .unwrap();
        });
    }
}
