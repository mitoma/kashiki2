use std::collections::HashMap;
use winit::event::{
    ElementState, Event, KeyboardInput, ModifiersState, VirtualKeyCode, WindowEvent,
};

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
pub struct KeyWithModifier {
    key: VirtualKeyCode,
    modifires: ModifiersState,
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone)]
pub enum ActionCategory {
    System,
    User(String),
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone)]
pub struct Action {
    pub name: String,
    pub category: ActionCategory,
}

impl Action {
    fn new(name: &str, category: ActionCategory) -> Action {
        Action {
            name: name.to_owned(),
            category,
        }
    }
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone)]
pub struct Stroke {
    keys: Vec<KeyWithModifier>,
}

pub struct KeyBind {
    stroke: Stroke,
    action: Action,
}

pub struct ActionStore {
    keybinds: HashMap<Stroke, Action>,
    current_modifier: ModifiersState,
}

impl Default for ActionStore {
    fn default() -> Self {
        let mut store = ActionStore {
            keybinds: HashMap::new(),
            current_modifier: ModifiersState::empty(),
        };
        store.keybinds.insert(
            Stroke {
                keys: vec![KeyWithModifier {
                    key: VirtualKeyCode::Escape,
                    modifires: ModifiersState::empty(),
                }],
            },
            Action::new("exit", ActionCategory::System),
        );
        store.keybinds.insert(
            Stroke {
                keys: vec![KeyWithModifier {
                    key: VirtualKeyCode::C,
                    modifires: ModifiersState::CTRL,
                }],
            },
            Action::new("exit", ActionCategory::System),
        );
        store.keybinds.insert(
            Stroke {
                keys: vec![KeyWithModifier {
                    key: VirtualKeyCode::P,
                    modifires: ModifiersState::SHIFT,
                }],
            },
            Action::new("exit", ActionCategory::System),
        );
        store
    }
}

impl ActionStore {
    pub fn winit_event_to_action(&mut self, event: &Event<()>) -> Option<Action> {
        match event {
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(keycode),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                if let Some(action) = self.keybinds.get(&Stroke {
                    keys: vec![KeyWithModifier {
                        key: *keycode,
                        modifires: self.current_modifier.clone(),
                    }],
                }) {
                    return Some(action.clone());
                }
                None
            }
            Event::WindowEvent {
                event: WindowEvent::ModifiersChanged(state),
                ..
            } => {
                self.current_modifier = state.clone();
                None
            }

            _ => None,
        }
    }
}
