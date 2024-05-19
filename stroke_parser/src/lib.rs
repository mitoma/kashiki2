pub mod action_store_parser;
pub mod keys;
pub mod pointing_device;

use crate::pointing_device::MousePoint;
use keys::KeyCode;
use log::warn;
use pointing_device::MouseAction;
use serde_derive::{Deserialize, Serialize};
use std::ops::Deref;
use winit::event::{ElementState, Event, KeyEvent, WindowEvent};

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
pub(crate) struct InputWithModifier {
    input: Input,
    modifires: keys::ModifiersState,
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
pub(crate) enum Input {
    Keyboard(KeyCode),
    Mouse(MouseAction),
}

impl InputWithModifier {
    pub(crate) fn new_key(
        key: keys::KeyCode,
        modifires: keys::ModifiersState,
    ) -> InputWithModifier {
        InputWithModifier {
            input: Input::Keyboard(key),
            modifires,
        }
    }

    pub(crate) fn new_mouse(
        mouse: pointing_device::MouseAction,
        modifires: keys::ModifiersState,
    ) -> InputWithModifier {
        InputWithModifier {
            input: Input::Mouse(mouse),
            modifires,
        }
    }
}

#[derive(Debug, PartialOrd, PartialEq, Clone, Serialize, Deserialize)]
pub enum Action {
    Command(CommandNamespace, CommandName, ActionArgument),
    Keytype(char),
    ImeEnable,
    ImeDisable,
    ImePreedit(String, Option<(usize, usize)>),
    ImeInput(String),
}

impl Action {
    pub fn with_argument(self, argument: Option<ActionArgument>) -> Action {
        match argument {
            Some(argument) => match self {
                Action::Command(namespace, name, _) => Action::Command(namespace, name, argument),
                _ => self,
            },
            None => self,
        }
    }
}

#[derive(Debug, PartialOrd, PartialEq, Clone, Serialize, Deserialize)]
pub enum ActionArgument {
    None,
    String(String),
    Integer(i32),
    Float(f32),
    Point((f32, f32)),
}

impl Action {
    pub fn new_command(namespace: &str, name: &str) -> Action {
        Action::Command(
            CommandNamespace::new(String::from(namespace)),
            CommandName::new(String::from(name)),
            ActionArgument::None,
        )
    }

    pub fn new_command_with_argument(namespace: &str, name: &str, argument: &str) -> Action {
        Action::Command(
            CommandNamespace::new(String::from(namespace)),
            CommandName::new(String::from(name)),
            ActionArgument::String(String::from(argument)),
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
pub(crate) struct Stroke {
    keys: Vec<InputWithModifier>,
}

impl Stroke {
    pub(crate) fn new(keys: Vec<InputWithModifier>) -> Stroke {
        Stroke { keys }
    }

    fn append_key(&mut self, key: InputWithModifier) {
        self.keys.push(key);
    }

    fn starts_with(&self, stroke: &Stroke) -> bool {
        self.keys.starts_with(&stroke.keys)
    }

    fn clear(&mut self) {
        self.keys.clear()
    }
}

#[derive(Debug, PartialOrd, PartialEq, Clone, Serialize, Deserialize)]
pub struct KeyBind {
    stroke: Stroke,
    action: Action,
}

impl KeyBind {
    pub(crate) fn new(stroke: Stroke, action: Action) -> KeyBind {
        KeyBind { stroke, action }
    }
}

pub struct ActionStore {
    keybinds: Vec<KeyBind>,
    current_modifier: keys::ModifiersState,
    current_stroke: Stroke,
    current_mouse: Option<MousePoint>,
}

impl Default for ActionStore {
    fn default() -> Self {
        ActionStore {
            keybinds: Vec::new(),
            current_modifier: keys::ModifiersState::NONE,
            current_stroke: Default::default(),
            current_mouse: None,
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
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key,
                        text,
                        ..
                    },
                ..
            } => {
                self.current_stroke.append_key(InputWithModifier {
                    input: Input::Keyboard(keys::KeyCode::from(logical_key)),
                    modifires: self.current_modifier,
                });

                if let Some(action) = self.get_action() {
                    self.current_stroke.clear();
                    return Some(action);
                }

                // ストロークの最中と判断された場合は文字は入力しない
                if self.in_stroke() {
                    return None;
                }

                self.current_stroke.clear();
                text.clone().map(|text| {
                    if text.len() == 1 {
                        Action::Keytype(text.chars().next().unwrap())
                    } else {
                        // 二文字以上の文字が一つの Keyboard Input で出てくることは想定していないが
                        warn!("text.len() != 1");
                        Action::ImeInput(text.to_string())
                    }
                })
            }
            WindowEvent::ModifiersChanged(state) => {
                self.current_modifier = keys::ModifiersState::from(*state);
                None
            }
            WindowEvent::Ime(ime) => match ime {
                winit::event::Ime::Enabled => Some(Action::ImeEnable),
                winit::event::Ime::Preedit(value, position) => {
                    Some(Action::ImePreedit(value.to_string(), *position))
                }
                winit::event::Ime::Commit(value) => Some(Action::ImeInput(value.to_string())),
                winit::event::Ime::Disabled => Some(Action::ImeDisable),
            },
            WindowEvent::CursorLeft { .. } => {
                self.current_mouse = None;
                None
            }
            WindowEvent::CursorMoved { position, .. } => {
                // 今のままだとマウスの移動に対するアクションが多量でセンシティブすぎる
                let from = self.current_mouse.take();
                self.current_mouse = Some(MousePoint {
                    x: position.x,
                    y: position.y,
                });
                if let (Some(mouse), Some(current)) = (from, self.current_mouse.as_ref()) {
                    let (action, gain) = current.calc_action_and_gain(&mouse);
                    self.get_action_by_mouse(action, Some(gain))
                } else {
                    None
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if *state == ElementState::Pressed {
                    self.get_action_by_mouse(
                        MouseAction::from(button),
                        self.current_mouse
                            .as_ref()
                            .map(|m| ActionArgument::Point((m.x as f32, m.y as f32))),
                    )
                } else {
                    None
                }
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                winit::event::MouseScrollDelta::LineDelta(x, y) => {
                    println!("LineDelta x: {}, y: {}", x, y);
                    let (action, gain) = if *x > 0.0 {
                        (MouseAction::WheelRight, x.abs())
                    } else if *x < 0.0 {
                        (MouseAction::WheelLeft, x.abs())
                    } else if *y > 0.0 {
                        (MouseAction::WheelUp, y.abs())
                    } else if *y < 0.0 {
                        (MouseAction::WheelDown, y.abs())
                    } else {
                        return None;
                    };
                    self.get_action_by_mouse(action, Some(ActionArgument::Float(gain)))
                }
                winit::event::MouseScrollDelta::PixelDelta(d) => {
                    println!("LineDelta d: {:?}", d);
                    // TODO PixelDelta が必要になるシーンは当分先になりそうなので特にまだ対応はしない
                    None
                }
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

    fn get_action_by_mouse(
        &self,
        mouse_action: MouseAction,
        action_argument: Option<ActionArgument>,
    ) -> Option<Action> {
        let stroke = Stroke::new(vec![InputWithModifier {
            input: Input::Mouse(mouse_action),
            modifires: self.current_modifier,
        }]);

        self.keybinds
            .iter()
            .find(|keybind| keybind.stroke == stroke)
            .map(|keybind| {
                let result = keybind.action.clone();
                result.with_argument(action_argument)
            })
    }

    fn in_stroke(&self) -> bool {
        for KeyBind { stroke, .. } in &self.keybinds {
            if stroke.starts_with(&self.current_stroke) {
                return true;
            }
        }
        false
    }
}
