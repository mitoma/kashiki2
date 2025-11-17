use serde_derive::{Deserialize, Serialize};

use crate::ActionArgument;

// ほんとうは MouseXxx みたいな名前にしているが本来は PointingDeviceXxx とかが適切かもしれない。
// しかし話がややこしくなるのでここでは MouseXxx としている。

#[derive(Debug, PartialOrd, PartialEq, Clone, Serialize, Deserialize)]
pub(crate) struct MousePoint {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

impl MousePoint {
    pub(crate) fn calc_action_and_gain(&self, from: &MousePoint) -> (MouseAction, ActionArgument) {
        let dx = from.x - self.x;
        let dy = from.y - self.y;
        if dx.abs() > dy.abs() {
            let gain = ActionArgument::Float(dx.abs() as f32);
            if dx > 0.0 {
                (MouseAction::MoveRight, gain)
            } else {
                (MouseAction::MoveLeft, gain)
            }
        } else {
            let gain = ActionArgument::Float(dy.abs() as f32);
            if dy > 0.0 {
                (MouseAction::MoveDown, gain)
            } else {
                (MouseAction::MoveUp, gain)
            }
        }
    }
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum MouseAction {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    ClickLeft,
    ClickRight,
    ClickMiddle,
    WheelUp,
    WheelDown,
    WheelLeft,
    WheelRight,
    Unknown,
}

impl From<&winit::event::MouseButton> for MouseAction {
    fn from(value: &winit::event::MouseButton) -> Self {
        match value {
            winit::event::MouseButton::Left => MouseAction::ClickLeft,
            winit::event::MouseButton::Right => MouseAction::ClickRight,
            winit::event::MouseButton::Middle => MouseAction::ClickMiddle,
            _ => MouseAction::Unknown,
        }
    }
}

impl From<&winit::event::ButtonSource> for MouseAction {
    fn from(value: &winit::event::ButtonSource) -> Self {
        match value {
            winit::event::ButtonSource::Mouse(mouse_button) => MouseAction::from(mouse_button),
            winit::event::ButtonSource::Touch { .. } => todo!(),
            winit::event::ButtonSource::TabletTool { .. } => todo!(),
            winit::event::ButtonSource::Unknown(_) => MouseAction::Unknown,
        }
    }
}
