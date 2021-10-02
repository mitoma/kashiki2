use stroke_parser::{action_store_parser, Action, ActionStore};
use text_buffer::action::{ApplyResult, BufferAction, BufferApplyer};
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
    let mut undo_list: Vec<ApplyResult> = Vec::default();

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
                let cloned_caret = caret.clone();
                let action = match &*name.to_string() {
                    "exit" => {
                        *control_flow = ControlFlow::Exit;
                        BufferAction::Noop(cloned_caret)
                    }
                    "return" => BufferAction::InsertEnter(cloned_caret),
                    "backspace" => BufferAction::Backspace(cloned_caret),
                    "delete" => BufferAction::Delete(cloned_caret),
                    "previous" => BufferAction::Previous(cloned_caret),
                    "next" => BufferAction::Next(cloned_caret),
                    "back" => BufferAction::Back(cloned_caret),
                    "forward" => BufferAction::Forward(cloned_caret),
                    "head" => BufferAction::Head(cloned_caret),
                    "last" => BufferAction::Last(cloned_caret),
                    "undo" => {
                        println!("undo!");
                        if let Some(reverse_action) = undo_list.pop() {
                            println!("has undo item!");
                            let result = BufferApplyer::apply_reserve_actions(
                                &mut text_buffer,
                                &reverse_action.reverse_action,
                            );
                            caret.move_to(result.caret.row, result.caret.col);
                        };
                        BufferAction::Noop(cloned_caret)
                    }
                    _ => BufferAction::Noop(cloned_caret),
                };
                let result = BufferApplyer::apply_action(&mut text_buffer, &action);
                caret.move_to(result.caret.row, result.caret.col);
                if !result.reverse_action.is_empty() {
                    undo_list.push(result);
                }
            }
            /*
                       Some(Action::Command(category, name)) if *category == "system" && *name == "exit" => {
                           *control_flow = ControlFlow::Exit
                       }
                       Some(Action::Command(category, name)) if *category == "system" && *name == "return" => {
                           text_buffer.insert_enter(&mut caret);
                       }
                       Some(Action::Command(category, name))
                           if *category == "system" && *name == "backspace" =>
                       {
                           text_buffer.backspace(&mut caret);
                       }
                       Some(Action::Keytype(c)) => {
                           text_buffer.insert_char(&mut caret, c);
                       }
                       Some(command) => println!("{:?}", command),
            */
            Some(Action::Command(_, _)) => {}
            Some(Action::Keytype(c)) => {
                let action = BufferAction::InsertChar(caret.clone(), c);
                let result = BufferApplyer::apply_action(&mut text_buffer, &action);
                caret.move_to(result.caret.row, result.caret.col);
                undo_list.push(result);
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

        println!("\n----\n{}", text_buffer.to_buffer_string());
        println!("undo_list:{:?}", undo_list);
    });
}
