mod camera;
mod font_texture;
mod model;
mod state;
mod text;
mod texture;

use futures::executor::block_on;
use stroke_parser::{action_store_parser, Action, ActionStore};

use text_buffer::action::EditorOperation;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::camera::CameraOperation;

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut state = block_on(state::State::new(&window));

    let mut editor = text_buffer::editor::Editor::default();

    let mut store: ActionStore = Default::default();

    let key_setting = include_str!("key-settings.txt");
    let keybinds = action_store_parser::parse_setting(String::from(key_setting));
    keybinds
        .iter()
        .for_each(|k| store.register_keybind(k.clone()));

    event_loop.run(move |event, _, control_flow| {
        match store.winit_event_to_action(&event) {
            // system
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
            // UI
            Some(Action::Command(category, name)) if *category == "ui" => {
                let action = match &*name.to_string() {
                    "up" => Some(CameraOperation::Up),
                    "down" => Some(CameraOperation::Down),
                    "left" => Some(CameraOperation::Left),
                    "right" => Some(CameraOperation::Right),
                    "forward" => Some(CameraOperation::Forward),
                    "backward" => Some(CameraOperation::Backward),
                    _ => None,
                };
                if let Some(op) = action {
                    state.send_camera_operation(&op);
                }
            }
            Some(Action::Command(_, _)) => {}
            Some(Action::Keytype(c)) => {
                let action = EditorOperation::InsertChar(c);
                editor.operation(&action);
            }
            None => {}
        }

        state.change_string(editor.to_buffer_string()); // FIXME

        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput { input, .. } => match input {
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    _ => {}
                },
                WindowEvent::Resized(physical_size) => {
                    state.resize(*physical_size);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    state.resize(**new_inner_size);
                }
                _ => {}
            },
            Event::RedrawRequested(_) => {
                state.update();
                state.render();
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => {}
        }
    });
}
