use stroke_parser::{Action, ActionStore, action_store_parser};
use text_buffer::{
    action::EditorOperation,
    editor::{ChangeEvent, Editor},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::EventLoop,
    window::{Window, WindowAttributes},
};

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();

    let (tx, _rx) = std::sync::mpsc::channel::<ChangeEvent>();
    let editor = text_buffer::editor::Editor::new(tx);

    let mut store: ActionStore = Default::default();
    let key_setting = include_str!("key-settings.txt");
    println!("{}", key_setting);
    let keybinds = action_store_parser::parse_setting(key_setting);
    keybinds
        .iter()
        .for_each(|k| store.register_keybind(k.clone()));

    println!("{}", store.keybinds_to_string());

    let mut state = State {
        window: None,
        store,
        editor,
    };
    let _ = event_loop.run_app(&mut state);
}

struct State {
    window: Option<Window>,
    store: ActionStore,
    editor: Editor,
}

impl ApplicationHandler for State {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(WindowAttributes::default().with_title("parser_test"))
            .unwrap();
        window.set_ime_allowed(true);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let wid = window_id;
        match self.store.winit_window_event_to_action(&event) {
            Some(Action::Command(category, name, _)) if *category == "system" => {
                let action = match &*name.to_string() {
                    "exit" => {
                        event_loop.exit();
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
                self.editor.operation(&action);
            }
            Some(Action::Command(_, _, _)) => {}
            Some(Action::Keytype(c)) => {
                let action = EditorOperation::InsertChar(c);
                self.editor.operation(&action);
            }
            Some(Action::ImeDisable) => {}
            Some(Action::ImeEnable) => {}
            Some(Action::ImePreedit(_, _)) => {}
            Some(Action::ImeInput(text)) => {
                let action = EditorOperation::InsertString(text);
                self.editor.operation(&action)
            }
            None => {}
        }
        match event {
            WindowEvent::CloseRequested if window_id == wid => event_loop.exit(),
            _ => (),
        }
        println!("\n----\n{}", self.editor.to_buffer_string());
    }
}
