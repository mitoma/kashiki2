use stroke_parser::{Action, ActionStore};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::EventLoop,
    window::{Window, WindowAttributes},
};

#[derive(Default)]
struct App {
    window: Option<Box<dyn Window>>,
    store: ActionStore,
}

impl ApplicationHandler for App {
    fn can_create_surfaces(&mut self, event_loop: &dyn winit::event_loop::ActiveEventLoop) {
        let window_attributes = WindowAttributes::default();
        self.window = match event_loop.create_window(window_attributes) {
            Ok(window) => Some(window),
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
            Some(Action::Command(category, name, _))
                if *category == "system" && *name == "exit" =>
            {
                event_loop.exit();
            }
            Some(command) => println!("{:?}", command),
            None => {}
        }

        let Some(self_window_id) = self.window.as_ref().map(|w| w.id()) else {
            return;
        };
        match event {
            WindowEvent::CloseRequested if window_id == self_window_id => event_loop.exit(),
            _ => (),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new().unwrap();

    let store = {
        let mut store: ActionStore = Default::default();
        let key_setting = include_str!("key-settings.txt");
        println!("{}", key_setting);
        let keybinds = stroke_parser::action_store_parser::parse_setting(key_setting);
        keybinds
            .iter()
            .for_each(|k| store.register_keybind(k.clone()));

        println!("{}", store.keybinds_to_string());
        store
    };

    let app = App {
        window: None,
        store,
    };

    event_loop.run_app(app)?;

    Ok(())
}
