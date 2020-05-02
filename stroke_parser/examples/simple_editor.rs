use stroke_parser::{keys, Action, ActionStore, KeyBind, KeyWithModifier, Stroke};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut caret = text_buffer::caret::Caret::new(0, 0);
    let mut text_buffer = text_buffer::buffer::Buffer::new();
    text_buffer.insert_string(&mut caret, "hello world".to_string());

    let mut store: ActionStore = Default::default();
    store.register_keybind(KeyBind::new(
        Stroke::new(vec![KeyWithModifier::new(
            keys::KeyCode::Return,
            keys::ModifiersState::NONE,
        )]),
        Action::new_command("system", "return"),
    ));

    println!("{}", store.keybinds_to_string());

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match store.winit_event_to_action(&event) {
            Some(Action::Command(category, name)) if *category == "system" && *name == "exit" => {
                *control_flow = ControlFlow::Exit
            }
            Some(Action::Command(category, name)) if *category == "system" && *name == "return" => {
                text_buffer.insert_enter(&mut caret);
            }
            Some(Action::Keytype(c)) => {
                text_buffer.insert_char(&mut caret, c);
                println!("{}", text_buffer.to_buffer_string());
            }
            Some(command) => println!("{:?}", command),
            None => {}
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            _ => (),
        }
    });
}
