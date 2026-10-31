pub mod action_store_parser;
pub mod keys;
pub mod pointing_device;

use crate::pointing_device::MousePoint;
use keys::KeyCode;
use log::warn;
use pointing_device::MouseAction;
use serde_derive::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    ops::Deref,
};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent, WindowEvent},
};

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
    String2(String, String),
    String3(String, String, String),
    String4(String, String, String, String),
    Integer(i32),
    Float(f32),
    Point((f32, f32)),
}

impl Display for ActionArgument {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            ActionArgument::None => write!(f, "None"),
            ActionArgument::String(value) => write!(f, "{}", value),
            ActionArgument::String2(v1, v2) => write!(f, "{}, {}", v1, v2),
            ActionArgument::String3(v1, v2, v3) => write!(f, "{}, {}, {}", v1, v2, v3),
            ActionArgument::String4(v1, v2, v3, v4) => write!(f, "{}, {}, {}, {}", v1, v2, v3, v4),
            ActionArgument::Integer(value) => write!(f, "{}", value),
            ActionArgument::Float(value) => write!(f, "{}", value),
            ActionArgument::Point((x, y)) => write!(f, "({}, {})", x, y),
        }
    }
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

impl From<&str> for CommandNamespace {
    fn from(value: &str) -> Self {
        Self::new(String::from(value))
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

impl From<&str> for CommandName {
    fn from(value: &str) -> Self {
        Self::new(String::from(value))
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
    current_namespaces: Option<Vec<CommandNamespace>>,
    current_modifier: keys::ModifiersState,
    current_stroke: Stroke,
    current_mouse: Option<MousePoint>,
}

impl Default for ActionStore {
    fn default() -> Self {
        ActionStore {
            keybinds: Vec::new(),
            current_namespaces: None,
            current_modifier: keys::ModifiersState::NONE,
            current_stroke: Default::default(),
            current_mouse: None,
        }
    }
}

impl ActionStore {
    /// 名前空間を優先順に指定する。指定外のコマンドは入力候補から除外する。
    ///
    /// 同じ名前空間内は登録順を優先する。名前空間の変更時は入力途中のストロークを消去する。
    /// 未設定なら従来どおりすべてのコマンドが有効で、空リストならコマンドを無効にする。
    /// 通常の文字入力と IME 入力には影響しない。
    ///
    /// ```
    /// use stroke_parser::ActionStore;
    ///
    /// let mut store = ActionStore::default();
    /// store.set_current_namespaces(["tagbase-memo", "edit", "world", "system"]);
    /// store.clear_current_namespaces();
    /// ```
    pub fn set_current_namespaces(
        &mut self,
        namespaces: impl IntoIterator<Item = impl Into<CommandNamespace>>,
    ) {
        let namespaces = namespaces.into_iter().map(Into::into).collect();
        if self.current_namespaces.as_ref() != Some(&namespaces) {
            self.current_namespaces = Some(namespaces);
            self.current_stroke.clear();
        }
    }

    /// 名前空間による制限を解除し、従来の登録順で解決する。
    pub fn clear_current_namespaces(&mut self) {
        self.current_namespaces = None;
        self.current_stroke.clear();
    }

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
                winit::event::Ime::DeleteSurrounding { .. } => {
                    // TODO: どのような挙動にするか考える必要がある
                    None
                }
            },
            WindowEvent::PointerLeft { .. } => {
                self.current_mouse = None;
                None
            }
            WindowEvent::PointerMoved { position, .. } => {
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
            WindowEvent::PointerButton { state, button, .. } => {
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
                winit::event::MouseScrollDelta::PixelDelta(PhysicalPosition { x, y }) => {
                    let (action, gain) = if *x > 0.0 {
                        (MouseAction::WheelRight, x.abs() as f32)
                    } else if *x < 0.0 {
                        (MouseAction::WheelLeft, x.abs() as f32)
                    } else if *y > 0.0 {
                        (MouseAction::WheelUp, y.abs() as f32)
                    } else if *y < 0.0 {
                        (MouseAction::WheelDown, y.abs() as f32)
                    } else {
                        return None;
                    };
                    self.get_action_by_mouse(action, Some(ActionArgument::Float(gain)))
                }
            },
            _ => None,
        }
    }

    pub fn winit_event_to_action(&mut self, event: &WindowEvent) -> Option<Action> {
        self.winit_window_event_to_action(event)
    }

    pub fn register_keybind(&mut self, keybind: KeyBind) {
        self.keybinds.push(keybind);
    }

    fn get_action(&self) -> Option<Action> {
        self.find_keybind(&self.current_stroke)
            .map(|keybind| keybind.action.clone())
    }

    fn namespace_priority(&self, action: &Action) -> Option<usize> {
        match (&self.current_namespaces, action) {
            (Some(namespaces), Action::Command(namespace, ..)) => {
                namespaces.iter().position(|active| active == namespace)
            }
            (Some(namespaces), _) => Some(namespaces.len()),
            (None, _) => Some(0),
        }
    }

    fn find_keybind(&self, stroke: &Stroke) -> Option<&KeyBind> {
        self.keybinds
            .iter()
            .filter(|keybind| keybind.stroke == *stroke)
            .filter_map(|keybind| {
                self.namespace_priority(&keybind.action)
                    .map(|priority| (priority, keybind))
            })
            .min_by_key(|(priority, _)| *priority)
            .map(|(_, keybind)| keybind)
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

        self.find_keybind(&stroke).map(|keybind| {
            let result = keybind.action.clone();
            result.with_argument(action_argument)
        })
    }

    fn in_stroke(&self) -> bool {
        self.keybinds.iter().any(|keybind| {
            self.namespace_priority(&keybind.action).is_some()
                && keybind.stroke.starts_with(&self.current_stroke)
        })
    }
}

#[cfg(test)]
mod action_store_tests {
    use super::*;

    fn store(settings: &str) -> ActionStore {
        let mut store = ActionStore::default();
        for keybind in action_store_parser::parse_setting(settings) {
            store.register_keybind(keybind);
        }
        store
    }

    fn set_stroke(store: &mut ActionStore, keys: &str) {
        store.current_stroke = action_store_parser::parse_setting(&format!("{keys} test:stroke"))
            .remove(0)
            .stroke;
    }

    #[test]
    fn defaults_to_registration_order_and_can_clear_namespace_selection() {
        let mut store = store("C-Z C category:add\nC-Z C tagbase:add");
        set_stroke(&mut store, "C-Z C");
        assert_eq!(
            store.get_action(),
            Some(Action::new_command("category", "add"))
        );
        store.set_current_namespaces(["tagbase"]);
        set_stroke(&mut store, "C-Z C");
        assert_eq!(
            store.get_action(),
            Some(Action::new_command("tagbase", "add"))
        );
        store.clear_current_namespaces();
        assert!(store.current_stroke.keys.is_empty());
        set_stroke(&mut store, "C-Z C");
        assert_eq!(
            store.get_action(),
            Some(Action::new_command("category", "add"))
        );
    }

    #[test]
    fn namespace_priority_precedes_registration_order_and_preserves_ties() {
        let mut store = store(
            "C-Z C shared:add\nC-Z C tagbase:first\nC-Z C tagbase:second\nC-X C-S shared:save",
        );
        store.set_current_namespaces(["tagbase", "shared"]);
        set_stroke(&mut store, "C-Z C");
        assert_eq!(
            store.get_action(),
            Some(Action::new_command("tagbase", "first"))
        );
        set_stroke(&mut store, "C-X C-S");
        assert_eq!(
            store.get_action(),
            Some(Action::new_command("shared", "save"))
        );
    }

    #[test]
    fn inactive_bindings_do_not_capture_stroke_prefixes() {
        let mut store = store("C-Z C category:add\nC-X C-S shared:save");
        store.set_current_namespaces(["tagbase", "shared"]);
        set_stroke(&mut store, "C-Z");
        assert!(!store.in_stroke());
        set_stroke(&mut store, "C-Z C");
        assert!(store.get_action().is_none());
        set_stroke(&mut store, "C-X");
        assert!(store.in_stroke());
    }

    #[test]
    fn changing_context_clears_the_pending_stroke_but_reapplying_does_not() {
        let mut store = store("C-Z C category:add\nC-Z C tagbase:add");
        store.set_current_namespaces(["category"]);
        set_stroke(&mut store, "C-Z");
        store.set_current_namespaces(["category"]);
        assert!(!store.current_stroke.keys.is_empty());
        store.set_current_namespaces(["tagbase"]);
        assert!(store.current_stroke.keys.is_empty());
    }

    #[test]
    fn mouse_bindings_follow_namespace_priority_and_keep_arguments() {
        let mut store = store("ClickLeft category:click\nClickLeft tagbase:click");
        store.set_current_namespaces(["tagbase", "category"]);
        let argument = ActionArgument::Point((12.0, 34.0));
        let mouse = store.keybinds[0].stroke.keys[0].input;
        let Input::Mouse(mouse) = mouse else {
            panic!("expected a mouse binding")
        };
        assert_eq!(
            store.get_action_by_mouse(mouse, Some(argument.clone())),
            Some(Action::new_command("tagbase", "click").with_argument(Some(argument)))
        );
        store.set_current_namespaces(["shared"]);
        assert!(store.get_action_by_mouse(mouse, None).is_none());
    }

    #[test]
    fn empty_namespace_selection_disables_commands_but_not_ime() {
        let mut store = store("C-Z C category:add");
        store.set_current_namespaces(std::iter::empty::<CommandNamespace>());
        set_stroke(&mut store, "C-Z C");
        assert!(store.get_action().is_none());
        assert!(!store.in_stroke());
        assert_eq!(
            store.winit_window_event_to_action(&WindowEvent::Ime(winit::event::Ime::Commit(
                "text".into()
            ))),
            Some(Action::ImeInput("text".to_owned()))
        );
    }

    #[test]
    fn non_command_bindings_are_not_filtered_by_namespace() {
        let mut store = store("C-Z C category:add");
        store.keybinds[0].action = Action::ImeInput("literal".to_owned());
        store.set_current_namespaces(std::iter::empty::<CommandNamespace>());
        set_stroke(&mut store, "C-Z");
        assert!(store.in_stroke());
        set_stroke(&mut store, "C-Z C");
        assert_eq!(
            store.get_action(),
            Some(Action::ImeInput("literal".to_owned()))
        );
    }
}
