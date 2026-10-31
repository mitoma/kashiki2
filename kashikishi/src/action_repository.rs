use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use stroke_parser::Action;

static MODE_ACTIONS: LazyLock<Vec<ActionDefinition>> =
    LazyLock::new(|| serde_json::from_str(include_str!("../asset/actions/mode.json")).unwrap());
static SYSTEM_ACTIONS: LazyLock<Vec<ActionDefinition>> =
    LazyLock::new(|| serde_json::from_str(include_str!("../asset/actions/system.json")).unwrap());
static EDIT_ACTIONS: LazyLock<Vec<ActionDefinition>> =
    LazyLock::new(|| serde_json::from_str(include_str!("../asset/actions/edit.json")).unwrap());
static WORLD_ACTIONS: LazyLock<Vec<ActionDefinition>> =
    LazyLock::new(|| serde_json::from_str(include_str!("../asset/actions/world.json")).unwrap());
static ACTION_RECORDER_ACTIONS: LazyLock<Vec<ActionDefinition>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../asset/actions/action-recorder.json")).unwrap()
});
static KASHIKISHI_ACTIONS: LazyLock<Vec<ActionDefinition>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../asset/actions/kashikishi.json")).unwrap()
});

pub enum ActionNamespace {
    Mode,
    Edit,
    System,
    World,
    Kashikishi,
    ActionRecorder,
    #[allow(dead_code)]
    Custom(String),
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
pub struct ActionDefinition {
    pub namespace: String,
    pub name: String,
    pub description: String,
}

impl ActionDefinition {
    pub fn to_action(&self) -> Action {
        Action::new_command(&self.namespace, &self.name)
    }
}

#[derive(Default)]
pub(crate) struct ActionRepository {}

impl ActionRepository {
    pub(crate) fn load_actions(&self, namespace: ActionNamespace) -> &[ActionDefinition] {
        match namespace {
            ActionNamespace::Mode => MODE_ACTIONS.as_slice(),
            ActionNamespace::System => SYSTEM_ACTIONS.as_slice(),
            ActionNamespace::Edit => EDIT_ACTIONS.as_slice(),
            ActionNamespace::World => WORLD_ACTIONS.as_slice(),
            ActionNamespace::Kashikishi => KASHIKISHI_ACTIONS.as_slice(),
            ActionNamespace::ActionRecorder => ACTION_RECORDER_ACTIONS.as_slice(),
            ActionNamespace::Custom(_) => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_memo_modes_are_available_in_the_command_palette() {
        let repository = ActionRepository::default();
        let actions = repository.load_actions(ActionNamespace::Mode);
        assert!(actions.iter().any(|action| action.name == "category"));
        assert!(actions.iter().any(|action| action.name == "tagbase"));
    }

    #[test]
    fn tagbase_actions_are_available_in_the_command_palette() {
        let repository = ActionRepository::default();
        let actions = repository.load_actions(ActionNamespace::Kashikishi);
        for name in [
            "search-ui",
            "list-all",
            "add-memo",
            "edit-title-ui",
            "edit-tags-ui",
            "delete-memo-ui",
        ] {
            assert!(
                actions
                    .iter()
                    .any(|action| action.namespace == "tagbase-memo" && action.name == name),
                "{name}"
            );
        }
    }

    #[test]
    fn category_actions_use_the_category_namespace() {
        let repository = ActionRepository::default();
        let actions = repository.load_actions(ActionNamespace::Kashikishi);
        for name in [
            "move-category-ui",
            "add-category-ui",
            "rename-category-select-ui",
            "remove-category-ui",
            "move-memo-ui",
        ] {
            assert!(
                actions
                    .iter()
                    .any(|action| action.namespace == "categorized-memo" && action.name == name),
                "{name}"
            );
        }
    }

    #[test]
    fn memo_keybindings_have_contextual_commands_and_shared_save() {
        use stroke_parser::action_store_parser::parse_setting;

        let keybinds = parse_setting(include_str!("../asset/key-settings.txt"));
        for setting in [
            "C-Z C categorized-memo:add-memo",
            "C-Z C tagbase-memo:add-memo",
            "C-Z K categorized-memo:remove-memo",
            "C-Z K tagbase-memo:delete-memo-ui",
            "C-X C M categorized-memo:move-category-ui",
            "C-X C-S kashikishi:save",
        ] {
            assert!(
                keybinds.contains(&parse_setting(setting).remove(0)),
                "{setting}"
            );
        }
    }
}
