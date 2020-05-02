use stroke_parser::{
    action_store_parser, keys, Action, ActionStore, KeyBind, KeyWithModifier, Stroke,
};
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
    let key_setting = include_str!("key-settings.txt");
    println!("{}", key_setting);
    let keybinds = action_store_parser::ActionStoreParser::parse_setting(String::from(key_setting));
    keybinds
        .iter()
        .for_each(|k| store.register_keybind(k.clone()));

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
