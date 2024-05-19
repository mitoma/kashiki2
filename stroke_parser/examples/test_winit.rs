use stroke_parser::{Action, ActionStore};
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut store: ActionStore = Default::default();

    event_loop
        .run(move |event, control_flow| {
            //control_flow.set_control_flow(control_flow) = ControlFlow::Wait;
            match store.winit_event_to_action(&event) {
                Some(Action::Command(category, name, _))
                    if *category == "system" && *name == "exit" =>
                {
                    control_flow.exit();
                }
                Some(command) => println!("{:?}", command),
                None => {}
            }

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    window_id,
                } if window_id == window.id() => control_flow.exit(),
                _ => (),
            }
        })
        .unwrap();
}
