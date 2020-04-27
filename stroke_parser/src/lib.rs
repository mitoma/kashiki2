use winit::event::{
    ElementState, Event, KeyboardInput, ModifiersState, VirtualKeyCode, WindowEvent,
};

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Copy, Clone)]
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

impl Default for Stroke {
    fn default() -> Self {
        Stroke { keys: Vec::new() }
    }
}

impl Stroke {
    fn append_key(&mut self, key: KeyWithModifier) {
        self.keys.push(key)
    }

    fn starts_with(&self, stroke: &Stroke) -> bool {
        self.keys.starts_with(&stroke.keys)
    }

    fn clear(&mut self) {
        self.keys.clear()
    }
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone)]
pub struct KeyBind {
    stroke: Stroke,
    action: Action,
}

pub struct ActionStore {
    keybinds: Vec<KeyBind>,
    current_modifier: ModifiersState,
    current_stroke: Stroke,
}

impl Default for ActionStore {
    fn default() -> Self {
        let mut store = ActionStore {
            keybinds: Vec::new(),
            current_modifier: ModifiersState::empty(),
            current_stroke: Default::default(),
        };
        store.keybinds.push(KeyBind {
            stroke: Stroke {
                keys: vec![KeyWithModifier {
                    key: VirtualKeyCode::Escape,
                    modifires: ModifiersState::empty(),
                }],
            },
            action: Action::new("exit", ActionCategory::System),
        });
        store.keybinds.push(KeyBind {
            stroke: Stroke {
                keys: vec![
                    KeyWithModifier {
                        key: VirtualKeyCode::X,
                        modifires: ModifiersState::CTRL,
                    },
                    KeyWithModifier {
                        key: VirtualKeyCode::C,
                        modifires: ModifiersState::CTRL,
                    },
                ],
            },
            action: Action::new("exit", ActionCategory::System),
        });
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
                if Self::is_modifire_key(keycode) {
                    return None;
                }

                self.current_stroke.append_key(KeyWithModifier {
                    key: *keycode,
                    modifires: self.current_modifier.clone(),
                });

                if let Some(action) = self.get_action() {
                    self.current_stroke.clear();
                    return Some(action.clone());
                }

                if !self.in_stroke() {
                    self.current_stroke.clear();
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

    #[inline]
    fn is_modifire_key(keycode: &VirtualKeyCode) -> bool {
        match *keycode {
            VirtualKeyCode::LControl
            | VirtualKeyCode::RControl
            | VirtualKeyCode::LAlt
            | VirtualKeyCode::RAlt
            | VirtualKeyCode::LShift
            | VirtualKeyCode::RShift => true,
            _ => false,
        }
    }
}
