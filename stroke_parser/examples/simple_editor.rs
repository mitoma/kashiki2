use stroke_parser::{action_store_parser, Action, ActionStore};
use text_buffer::action::EditorOperation;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut editor = text_buffer::editor::Editor::default();

    let mut store: ActionStore = Default::default();
    let key_setting = include_str!("key-settings.txt");
    println!("{}", key_setting);
    let keybinds = action_store_parser::parse_setting(String::from(key_setting));
    keybinds
        .iter()
        .for_each(|k| store.register_keybind(k.clone()));

    println!("{}", store.keybinds_to_string());

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match store.winit_event_to_action(&event) {
            Some(Action::Command(category, name)) if *category == "system" => {
                let action = match &*name.to_string() {
                    "exit" => {
                        *control_flow = ControlFlow::Exit;
                        EditorOperation::Noop
                    }
                    "return" => EditorOperation::InsertEnter,
                    "backspace" => EditorOperation::Backspace,
                    "delete" => EditorOperation::Delete,
                    "previous" => EditorOperation::Previous,
                    "next" => EditorOperation::Next,
                    "back" => EditorOperation::Back,
                    "forward" => EditorOperation::Forward,
                    "head" => EditorOperation::Head,
                    "last" => EditorOperation::Last,
                    "undo" => EditorOperation::Undo,
                    _ => EditorOperation::Noop,
                };
                editor.operation(&action);
            }
            Some(Action::Command(_, _)) => {}
            Some(Action::Keytype(c)) => {
                let action = EditorOperation::InsertChar(c);
                editor.operation(&action);
            }
            None => {}
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            _ => (),
        }

        println!("\n----\n{}", editor.to_buffer_string());
    });
}
