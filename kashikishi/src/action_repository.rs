use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use stroke_parser::Action;

static SYSTEM_ACTIONS: LazyLock<Vec<ActionDefinition>> =
    LazyLock::new(|| serde_json::from_str(include_str!("../asset/actions/system.json")).unwrap());
static EDIT_ACTIONS: LazyLock<Vec<ActionDefinition>> =
    LazyLock::new(|| serde_json::from_str(include_str!("../asset/actions/edit.json")).unwrap());
static WORLD_ACTIONS: LazyLock<Vec<ActionDefinition>> =
    LazyLock::new(|| serde_json::from_str(include_str!("../asset/actions/world.json")).unwrap());

pub enum ActionNamespace {
    Edit,
    System,
    World,
    Custom(String),
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
pub struct ActionDefinition {
    pub namespace: String,
    pub name: String,
    pub description: String,
}

impl ActionDefinition {
    pub fn new(namespace: String, name: String, description: String) -> Self {
        Self {
            namespace,
            name,
            description,
        }
    }

    pub fn to_action(&self) -> Action {
        Action::new_command(&self.namespace, &self.name)
    }
}

#[derive(Default)]
pub(crate) struct ActionRepository {}

impl ActionRepository {
    pub(crate) fn load_actions(&self, namespace: ActionNamespace) -> &[ActionDefinition] {
        match namespace {
            ActionNamespace::System => SYSTEM_ACTIONS.as_slice(),
            ActionNamespace::Edit => EDIT_ACTIONS.as_slice(),
            ActionNamespace::World => WORLD_ACTIONS.as_slice(),
            ActionNamespace::Custom(_) => todo!(),
        }
    }
}
