use std::sync::OnceLock;

use regex::Regex;
use web_time::Duration;

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
        static REGEX: OnceLock<Regex> = OnceLock::new();
        let re = REGEX.get_or_init(|| Regex::new(r"(<[a-zA-Z0-9]+>|.)").unwrap());

        let mut actions = target_string
            .lines()
            .flat_map(|line| {
                let tokens: Vec<&str> = re.find_iter(line).map(|m| m.as_str()).collect();
                tokens
                    .into_iter()
                    .flat_map(|token| match token {
                        "<bs>" | "<backspace>" => {
                            vec![
                                Action::Command(
                                    "edit".into(),
                                    "backspace".into(),
                                    ActionArgument::None,
                                ),
                                wait(200),
                            ]
                        }
                        "<enter>" | "<return>" => {
                            vec![enter(), wait(500)]
                        }
                        "<tate>" | "<vert>" | "<vertical>" => {
                            vec![direction_vertical(), wait(200)]
                        }
                        "<yoko>" | "<hori>" | "<horizontal>" => {
                            vec![direction_horizontal(), wait(200)]
                        }
                        _ => token
                            .chars()
                            .flat_map(|c| [Action::Keytype(c), wait(200)])
                            .collect(),
                    })
                    .chain([enter(), wait(500)])
            })
            .collect();
        self.actions.append(&mut actions);
    }

    pub fn all_time_frames(&self) -> Duration {
        self.actions.iter().fold(Duration::ZERO, |acc, action| {
            if let Action::Command(namespace, cmd, ActionArgument::Integer(frames)) = action
                && namespace.to_string() == "action_recorder"
                && cmd.to_string() == "wait"
            {
                acc + Duration::from_millis(*frames as u64)
            } else {
                acc
            }
        })
    }
}

fn wait(millis: u32) -> Action {
    Action::Command(
        "action_recorder".into(),
        "wait".into(),
        ActionArgument::Integer(millis as i32),
    )
}

fn enter() -> Action {
    Action::Command("edit".into(), "return".into(), ActionArgument::None)
}

fn direction_vertical() -> Action {
    Action::new_command_with_argument("system", "change-global-direction", "Vertical")
}

fn direction_horizontal() -> Action {
    Action::new_command_with_argument("system", "change-global-direction", "Horizontal")
}

impl ActionRecordRepository for ActionRecordConverter {
    fn save(&mut self, _action: &[Action]) {
        todo!()
    }

    fn load(&self) -> Vec<Action> {
        self.actions.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_record_converter() {
        let pattern = Regex::new(r"(<[a-zA-Z0-9]+>|.)").unwrap();
        let tokens: Vec<&str> = pattern
            .find_iter("hello<bs>world")
            .map(|m| m.as_str())
            .collect();
        println!("{:?}", tokens);
    }
}
