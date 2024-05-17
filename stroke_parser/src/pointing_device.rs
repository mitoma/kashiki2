use serde_derive::{Deserialize, Serialize};

// ほんとうは MouseXxx みたいな名前にしているが本来は PointingDeviceXxx とかが適切かもしれない。
// しかし話がややこしくなるのでここでは MouseXxx としている。

#[derive(Debug, PartialOrd, PartialEq, Clone, Serialize, Deserialize)]
pub(crate) struct MousePoint {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

impl MousePoint {
    pub(crate) fn mouse_move(&self, from: &MousePoint) -> MouseAction {
        let dx = from.x - self.x;
        let dy = from.y - self.y;
        if dx.abs() > dy.abs() {
            if dx > 0.0 {
                MouseAction::MoveRight
            } else {
                MouseAction::MoveLeft
            }
        } else if dy > 0.0 {
            MouseAction::MoveDown
        } else {
            MouseAction::MoveUp
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
