pub mod action_store_parser;
pub mod keys;

use serde_derive::{Deserialize, Serialize};
use std::ops::Deref;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
pub struct KeyWithModifier {
    key: keys::KeyCode,
    modifires: keys::ModifiersState,
}

impl KeyWithModifier {
    pub fn new(key: keys::KeyCode, modifires: keys::ModifiersState) -> KeyWithModifier {
        KeyWithModifier { key, modifires }
    }
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum Action {
    Command(CommandNamespace, CommandName),
    Keytype(char),
    ImeEnable,
    ImeDisable,
    ImePreedit(String, Option<(usize, usize)>),
    ImeInput(String),
}

impl Action {
    pub fn new_command(namespace: &str, name: &str) -> Action {
        Action::Command(
            CommandNamespace::new(String::from(namespace)),
            CommandName::new(String::from(name)),
        )
    }
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CommandNamespace(String);

impl Deref for CommandNamespace {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl CommandNamespace {
    pub fn new(value: String) -> CommandNamespace {
        CommandNamespace(value)
    }
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CommandName(String);

impl Deref for CommandName {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl CommandName {
    pub fn new(value: String) -> CommandName {
        CommandName(value)
    }
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
pub struct Stroke {
    keys: Vec<KeyWithModifier>,
}

impl Stroke {
    pub fn new(keys: Vec<KeyWithModifier>) -> Stroke {
        Stroke { keys }
    }

    fn append_key(&mut self, key: KeyWithModifier) {
        self.keys.push(key);
    }

    fn starts_with(&self, stroke: &Stroke) -> bool {
        self.keys.starts_with(&stroke.keys)
    }

    fn clear(&mut self) {
        self.keys.clear()
    }
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct KeyBind {
    stroke: Stroke,
    action: Action,
}

impl KeyBind {
    pub fn new(stroke: Stroke, action: Action) -> KeyBind {
        KeyBind { stroke, action }
    }
}

pub struct ActionStore {
    keybinds: Vec<KeyBind>,
    current_modifier: keys::ModifiersState,
    current_stroke: Stroke,
}

impl Default for ActionStore {
    fn default() -> Self {
        ActionStore {
            keybinds: Vec::new(),
            current_modifier: keys::ModifiersState::NONE,
            current_stroke: Default::default(),
        }
    }
}

impl ActionStore {
    pub fn keybinds_to_string(&self) -> String {
        serde_json::to_string(&self.keybinds).unwrap()
    }

    pub fn winit_window_event_to_action(&mut self, event: &WindowEvent) -> Option<Action> {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => {
                if Self::is_modifire_key(keycode) {
                    return None;
                }

                self.current_stroke.append_key(KeyWithModifier {
                    key: keys::KeyCode::from(*keycode),
                    modifires: self.current_modifier,
                });

                if let Some(action) = self.get_action() {
                    self.current_stroke.clear();
                    return Some(action);
                }

                if !self.in_stroke() {
                    self.current_stroke.clear();
                }
                None
            }
            WindowEvent::ModifiersChanged(state) => {
                self.current_modifier = keys::ModifiersState::from(*state);
                None
            }
            WindowEvent::ReceivedCharacter(c) => {
                if c.is_control() {
                    // Enter や Backspace は Action で対応する？
                    None
                } else {
                    Some(Action::Keytype(*c))
                }
            }
            WindowEvent::Ime(ime) => match ime {
                winit::event::Ime::Enabled => Some(Action::ImeEnable),
                winit::event::Ime::Preedit(value, position) => {
                    Some(Action::ImePreedit(value.to_string(), position.clone()))
                }
                winit::event::Ime::Commit(value) => Some(Action::ImeInput(value.to_string())),
                winit::event::Ime::Disabled => Some(Action::ImeDisable),
            },
            _ => None,
        }
    }

    pub fn winit_event_to_action(&mut self, event: &Event<()>) -> Option<Action> {
        match event {
            Event::WindowEvent { event, .. } => self.winit_window_event_to_action(event),
            _ => None,
        }
    }

    pub fn register_keybind(&mut self, keybind: KeyBind) {
        self.keybinds.push(keybind);
    }

    fn get_action(&self) -> Option<Action> {
        self.keybinds
            .iter()
            .find(|keybind| keybind.stroke == self.current_stroke)
            .map(|keybind| keybind.action.clone())
    }

    fn in_stroke(&self) -> bool {
        for KeyBind { stroke, .. } in &self.keybinds {
            if stroke.starts_with(&self.current_stroke) {
                return true;
            }
        }
        false
    }

    fn is_modifire_key(keycode: &VirtualKeyCode) -> bool {
        matches!(
            *keycode,
            VirtualKeyCode::LControl
                | VirtualKeyCode::RControl
                | VirtualKeyCode::LAlt
                | VirtualKeyCode::RAlt
                | VirtualKeyCode::LShift
                | VirtualKeyCode::RShift
        )
    }
}
