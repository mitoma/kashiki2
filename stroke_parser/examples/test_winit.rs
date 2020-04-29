use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use stroke_parser::{Action, ActionStore};

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut store: ActionStore = Default::default();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match store.winit_event_to_action(&event) {
            Some(Action::Command(category, name)) if *category == "system" && *name == "exit" => {
                *control_flow = ControlFlow::Exit
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
