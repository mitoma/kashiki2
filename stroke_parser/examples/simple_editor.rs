use stroke_parser::{action_store_parser, Action, ActionStore};
use text_buffer::{action::EditorOperation, editor::ChangeEvent};
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    window.set_ime_allowed(true);

    let (tx, _rx) = std::sync::mpsc::channel::<ChangeEvent>();
    let mut editor = text_buffer::editor::Editor::new(tx);

    let mut store: ActionStore = Default::default();
    let key_setting = include_str!("key-settings.txt");
    println!("{}", key_setting);
    let keybinds = action_store_parser::parse_setting(String::from(key_setting));
    keybinds
        .iter()
        .for_each(|k| store.register_keybind(k.clone()));

    println!("{}", store.keybinds_to_string());

    event_loop
        .run(move |event, control_flow| {
            match store.winit_event_to_action(&event) {
                Some(Action::Command(category, name)) if *category == "system" => {
                    let action = match &*name.to_string() {
                        "exit" => {
                            control_flow.exit();
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
                Some(Action::ImeDisable) => {}
                Some(Action::ImeEnable) => {}
                Some(Action::ImePreedit(_, _)) => {}
                Some(Action::ImeInput(text)) => {
                    let action = EditorOperation::InsertString(text);
                    editor.operation(&action)
                }
                None => {}
            }

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    window_id,
                } if window_id == window.id() => control_flow.exit(),
                _ => (),
            }

            println!("\n----\n{}", editor.to_buffer_string());
        })
        .unwrap();
}
