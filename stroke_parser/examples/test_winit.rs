use stroke_parser::{Action, ActionStore, action_store_parser};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::EventLoop,
    window::{Window, WindowAttributes},
};

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let mut store: ActionStore = Default::default();
    let key_setting = include_str!("key-settings.txt");
    println!("{}", key_setting);
    let keybinds = action_store_parser::parse_setting(key_setting);
    keybinds
        .iter()
        .for_each(|k| store.register_keybind(k.clone()));

    let mut state = State {
        window: None,
        store,
    };
    let _ = event_loop.run_app(&mut state);
}

struct State {
    window: Option<Window>,
    store: ActionStore,
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
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match self.store.winit_window_event_to_action(&event) {
            Some(Action::Command(category, name, _))
                if *category == "system" && *name == "exit" =>
            {
                event_loop.exit();
            }
            Some(command) => println!("{:?}", command),
            None => {}
        }
    }
}
