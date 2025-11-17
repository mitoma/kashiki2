use stroke_parser::{Action, ActionStore, action_store_parser};
use text_buffer::{
    action::EditorOperation,
    editor::{ChangeEvent, Editor},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::EventLoop,
    window::{ImeCapabilities, ImeEnableRequest, ImeRequestData, Window, WindowAttributes},
};

struct App {
    window: Option<Box<dyn Window>>,
    editor: Editor,
    store: ActionStore,
}

impl ApplicationHandler for App {
    fn can_create_surfaces(&mut self, event_loop: &dyn winit::event_loop::ActiveEventLoop) {
        let window_attributes = WindowAttributes::default();
        self.window = match event_loop.create_window(window_attributes) {
            Ok(window) => {
                let req =
                    ImeEnableRequest::new(ImeCapabilities::default(), ImeRequestData::default())
                        .unwrap();
                let _ = window.request_ime_update(winit::window::ImeRequest::Enable(req));
                Some(window)
            }
            Err(err) => {
                eprintln!("error creating window: {err}");
                event_loop.exit();
                return;
            }
        };
    }

    fn window_event(
        &mut self,
        event_loop: &dyn winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match self.store.winit_event_to_action(&event) {
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

        let Some(self_window_id) = self.window.as_ref().map(|w| w.id()) else {
            return;
        };
        match event {
            WindowEvent::CloseRequested if window_id == self_window_id => event_loop.exit(),
            _ => (),
        }

        println!("\n----\n{}", self.editor.to_buffer_string());
    }
}

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

    let app = App {
        window: None,
        editor,
        store,
    };

    let _ = event_loop.run_app(app);
}
