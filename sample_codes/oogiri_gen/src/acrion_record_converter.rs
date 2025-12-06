use std::time::Duration;

use stroke_parser::{Action, ActionArgument};
use ui_support::action_recorder::ActionRecordRepository;

pub(crate) struct ActionRecordConverter {
    actions: Vec<Action>,
}

impl ActionRecordConverter {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    pub fn set_direction_vertical(&mut self) {
        self.actions.push(direction_vertical())
    }

    pub fn append(&mut self, target_string: &str) {
        let mut actions = target_string
            .lines()
            .flat_map(|line| {
                line.chars()
                    .flat_map(|c| [Action::Keytype(c), wait(200)])
                    .chain([enter(), wait(500)])
            })
            .collect();
        self.actions.append(&mut actions);
    }

    pub fn all_time_frames(&self) -> Duration {
        self.actions.iter().fold(Duration::ZERO, |acc, action| {
            if let Action::Command(namespace, cmd, arg) = action {
                if namespace.to_string() == "action_recorder" && cmd.to_string() == "wait" {
                    if let ActionArgument::Integer(frames) = arg {
                        return acc + Duration::from_millis(*frames as u64);
                    }
                }
            }
            acc
        })
    }
}

fn wait(frames: u32) -> Action {
    Action::Command(
        "action_recorder".into(),
        "wait".into(),
        ActionArgument::Integer(frames as i32),
    )
}

fn enter() -> Action {
    Action::Command("edit".into(), "return".into(), ActionArgument::None)
}

fn direction_vertical() -> Action {
    Action::new_command_with_argument("system", "change-global-direction", "Vertical")
}

impl ActionRecordRepository for ActionRecordConverter {
    fn save(&mut self, _action: &[Action]) {
        todo!()
    }

    fn load(&self) -> Vec<Action> {
        self.actions.clone()
    }
}
