use crate::{keys, Action, KeyBind, KeyWithModifier, Stroke};

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
        let strokes: Vec<KeyWithModifier> = settings
            .iter()
            .flat_map(|s| parse_keywithmodifier(s))
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
    Some(Action::new_command(words[0], words[1]))
}

fn parse_keywithmodifier(line: &str) -> Option<KeyWithModifier> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    if line.starts_with('#') {
        return None;
    }

    let key = line.rsplit('-').next().ok_or(()).and_then(|command| {
        serde_json::from_str::<keys::KeyCode>(&format!("\"{}\"", command)).map_err(|_| ())
    });
    let key = match key {
        Ok(key) => key,
        _ => return None,
    };

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

    Some(KeyWithModifier::new(key, modifires))
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn parse_setting_test() {
        assert_eq!(
            parse_setting(
                r"
                #hello
                Return system:enter
                C-X C-S system:save
            "
            ),
            vec![
                KeyBind::new(
                    Stroke::new(vec![KeyWithModifier::new(
                        keys::KeyCode::Return,
                        keys::ModifiersState::NONE
                    )]),
                    Action::new_command("system", "enter")
                ),
                KeyBind::new(
                    Stroke::new(vec![
                        KeyWithModifier::new(keys::KeyCode::X, keys::ModifiersState::Ctrl),
                        KeyWithModifier::new(keys::KeyCode::S, keys::ModifiersState::Ctrl)
                    ]),
                    Action::new_command("system", "save")
                )
            ]
        );
    }

    #[test]
    fn parse_keywithmodifier_none() {
        assert_eq!(parse_keywithmodifier(""), None);
        assert_eq!(parse_keywithmodifier("     "), None);
        assert_eq!(parse_keywithmodifier("# is comment line"), None);
    }

    #[test]
    fn parse_keywithmodifier_ok() {
        assert_eq!(
            parse_keywithmodifier("C-A-S-Return").unwrap(),
            KeyWithModifier::new(keys::KeyCode::Return, keys::ModifiersState::CtrlAltShift)
        );
        assert_eq!(
            parse_keywithmodifier("Return").unwrap(),
            KeyWithModifier::new(keys::KeyCode::Return, keys::ModifiersState::NONE)
        );
        assert_eq!(
            parse_keywithmodifier("S-C").unwrap(),
            KeyWithModifier::new(keys::KeyCode::C, keys::ModifiersState::Shift)
        );
        assert_eq!(
            parse_keywithmodifier("A-S-X").unwrap(),
            KeyWithModifier::new(keys::KeyCode::X, keys::ModifiersState::AltShift)
        );
    }
}
