use std::sync::LazyLock;

use cgmath::Quaternion;
use font_rasterizer::context::StateContext;
use rokid_3dof::RokidMax;
use stroke_parser::{ActionArgument, CommandName, CommandNamespace};
use ui_support::{
    action::NamespaceActionProcessors, camera::CameraOperation, layout_engine::World, InputResult,
};

static NAMES: LazyLock<Vec<CommandName>> =
    LazyLock::new(|| vec!["reset".into(), "toggle-mode".into()]);

pub struct RokidMaxAction {
    rokid_max: Option<RokidMax>,
    ar_mode: bool,
}

impl RokidMaxAction {
    pub fn new() -> Self {
        Self {
            rokid_max: RokidMax::new().ok(),
            ar_mode: false,
        }
    }

    pub fn reset(&mut self) {
        let _ = self.rokid_max.as_mut().and_then(|r| r.reset().ok());
    }

    pub fn quaternion(&self) -> Option<Quaternion<f32>> {
        if !self.ar_mode {
            return None;
        }
        self.rokid_max.as_ref().map(|r| r.quaternion())
    }
}

impl NamespaceActionProcessors for RokidMaxAction {
    fn namespace(&self) -> CommandNamespace {
        "rokid-max".into()
    }

    fn names(&self) -> &[CommandName] {
        NAMES.as_slice()
    }

    fn process(
        &mut self,
        command_name: &CommandName,
        _arg: &ActionArgument,
        _context: &StateContext,
        world: &mut dyn World,
    ) -> InputResult {
        match command_name.as_str() {
            "reset" => {
                if let Some(rokid_max) = self.rokid_max.as_mut() {
                    let _ = rokid_max.reset();
                }
            }
            "toggle-mode" => {
                if let Some(rokid_max) = self.rokid_max.as_mut() {
                    let _ = rokid_max.reset();
                    world.camera_operation(CameraOperation::UpdateEyeQuaternion(Some(
                        rokid_max.quaternion(),
                    )));
                }
                self.ar_mode = !self.ar_mode;
            }
            _ => {}
        }
        InputResult::InputConsumed
    }
}
