use crate::{Action, InputWithModifier, KeyBind, Stroke, keys, pointing_device};

pub fn parse_setting(setting_string: &str) -> Vec<KeyBind> {
    let mut result: Vec<KeyBind> = Vec::new();
    for line in setting_string.lines() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            continue;
        }

        let mut settings: Vec<&str> = line.split(' ').collect();
        let command = settings.pop().and_then(parse_action);
        let strokes: Vec<InputWithModifier> = settings
            .iter()
            .flat_map(|s| parse_input_with_modifier(s))
            .collect();
        if let Some(command) = command {
            result.push(KeyBind::new(Stroke::new(strokes), command))
        }
    }
    result
}

fn parse_action(line: &str) -> Option<Action> {
    let words: Vec<&str> = line.split(':').collect();
    if words.len() != 2 {
        return None;
    }

    let namespace = words[0];
    if words[1].contains('(') && words[1].ends_with(')') {
        let words: Vec<&str> = words[1].split('(').collect();
        let name = words[0];
        let argument = words[1].trim_end_matches(')');
        Some(Action::new_command_with_argument(namespace, name, argument))
    } else {
        let name = words[1];
        Some(Action::new_command(namespace, name))
    }
}

fn parse_input_with_modifier(line: &str) -> Option<InputWithModifier> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    if line.starts_with('#') {
        return None;
    }

    let modifires = if line.starts_with("C-A-S-") {
        keys::ModifiersState::CtrlAltShift
    } else if line.starts_with("C-A-") {
        keys::ModifiersState::CtrlAlt
    } else if line.starts_with("C-S-") {
        keys::ModifiersState::CtrlShift
    } else if line.starts_with("A-S-") {
        keys::ModifiersState::AltShift
    } else if line.starts_with("C-") {
        keys::ModifiersState::Ctrl
    } else if line.starts_with("A-") {
        keys::ModifiersState::Alt
    } else if line.starts_with("S-") {
        keys::ModifiersState::Shift
    } else {
        keys::ModifiersState::NONE
    };

    let command_token = line.rsplit('-').next();

    let key = command_token.ok_or(()).and_then(|command| {
        serde_json::from_str::<keys::KeyCode>(&format!("\"{}\"", command)).map_err(|_| ())
    });
    let mouse = command_token.ok_or(()).and_then(|command| {
        serde_json::from_str::<pointing_device::MouseAction>(&format!("\"{}\"", command))
            .map_err(|_| ())
    });

    match (key, mouse) {
        (Ok(key), _) => Some(InputWithModifier::new_key(key, modifires)),
        (_, Ok(mouse)) => Some(InputWithModifier::new_mouse(mouse, modifires)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {

    use crate::pointing_device;

    use super::*;

    #[test]
    fn parse_setting_test() {
        assert_eq!(
            parse_setting(
                r"
                #hello
                Return system:enter
                C-X C-S system:save
                C-X C-T C-D system:change-theme(dark)
                # Argument が空文字のケース
                C-X C-T C-E system:change-theme()
                # 日本語のargumentも特に禁止しない
                C-A system:author(山田太郎的な存在)
            "
            ),
            vec![
                KeyBind::new(
                    Stroke::new(vec![InputWithModifier::new_key(
                        keys::KeyCode::Return,
                        keys::ModifiersState::NONE
                    )]),
                    Action::new_command("system", "enter")
                ),
                KeyBind::new(
                    Stroke::new(vec![
                        InputWithModifier::new_key(keys::KeyCode::X, keys::ModifiersState::Ctrl),
                        InputWithModifier::new_key(keys::KeyCode::S, keys::ModifiersState::Ctrl)
                    ]),
                    Action::new_command("system", "save")
                ),
                KeyBind::new(
                    Stroke::new(vec![
                        InputWithModifier::new_key(keys::KeyCode::X, keys::ModifiersState::Ctrl),
                        InputWithModifier::new_key(keys::KeyCode::T, keys::ModifiersState::Ctrl),
                        InputWithModifier::new_key(keys::KeyCode::D, keys::ModifiersState::Ctrl)
                    ]),
                    Action::new_command_with_argument("system", "change-theme", "dark")
                ),
                KeyBind::new(
                    Stroke::new(vec![
                        InputWithModifier::new_key(keys::KeyCode::X, keys::ModifiersState::Ctrl),
                        InputWithModifier::new_key(keys::KeyCode::T, keys::ModifiersState::Ctrl),
                        InputWithModifier::new_key(keys::KeyCode::E, keys::ModifiersState::Ctrl)
                    ]),
                    Action::new_command_with_argument("system", "change-theme", "")
                ),
                KeyBind::new(
                    Stroke::new(vec![InputWithModifier::new_key(
                        keys::KeyCode::A,
                        keys::ModifiersState::Ctrl
                    ),]),
                    Action::new_command_with_argument("system", "author", "山田太郎的な存在")
                )
            ]
        );
    }

    #[test]
    fn parse_input_with_modifier_none() {
        assert_eq!(parse_input_with_modifier(""), None);
        assert_eq!(parse_input_with_modifier("     "), None);
        assert_eq!(parse_input_with_modifier("# is comment line"), None);
    }

    #[test]
    fn parse_input_with_modifier_ok() {
        assert_eq!(
            parse_input_with_modifier("C-A-S-Return").unwrap(),
            InputWithModifier::new_key(keys::KeyCode::Return, keys::ModifiersState::CtrlAltShift)
        );
        assert_eq!(
            parse_input_with_modifier("Return").unwrap(),
            InputWithModifier::new_key(keys::KeyCode::Return, keys::ModifiersState::NONE)
        );
        assert_eq!(
            parse_input_with_modifier("S-C").unwrap(),
            InputWithModifier::new_key(keys::KeyCode::C, keys::ModifiersState::Shift)
        );
        assert_eq!(
            parse_input_with_modifier("A-S-X").unwrap(),
            InputWithModifier::new_key(keys::KeyCode::X, keys::ModifiersState::AltShift)
        );
    }

    #[test]
    fn parse_input_with_modifier_mouse() {
        assert_eq!(
            parse_input_with_modifier("C-A-S-MoveLeft").unwrap(),
            InputWithModifier::new_mouse(
                pointing_device::MouseAction::MoveLeft,
                keys::ModifiersState::CtrlAltShift
            )
        );
        assert_eq!(
            parse_input_with_modifier("ClickMiddle").unwrap(),
            InputWithModifier::new_mouse(
                pointing_device::MouseAction::ClickMiddle,
                keys::ModifiersState::NONE
            )
        );
        assert_eq!(
            parse_input_with_modifier("WheelUp").unwrap(),
            InputWithModifier::new_mouse(
                pointing_device::MouseAction::WheelUp,
                keys::ModifiersState::NONE
            )
        );
    }
}
