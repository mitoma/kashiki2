use std::{path::Path, sync::LazyLock};

use stroke_parser::{Action, ActionArgument};

use crate::{
    layout_engine::Model,
    ui::{SelectBox, SelectOption},
    ui_context::UiContext,
};

pub struct FileChooser {
    select_box: SelectBox,
}

fn set_path(path: &Path, recursion_action: &Action) -> Action {
    let Action::Command(_namespace, _name, ActionArgument::String4(_v1, v2, v3, v4)) =
        recursion_action.clone()
    else {
        panic!(
            "recursion_action should be a command action with 3 string arguments (file path, message, original target action namespace, original target action name)"
        );
    };
    recursion_action
        .clone()
        .with_argument(Some(ActionArgument::String4(
            path.to_str().unwrap().to_string(),
            v2,
            v3,
            v4,
        )))
}

fn file_options(path: &Path, recursion_action: Action, target_action: Action) -> Vec<SelectOption> {
    if !path.is_dir() {
        vec![]
    } else {
        let mut options = vec![];
        if let Some(parent_dir) = path.parent() {
            options.push(SelectOption::new(
                "📁 ../".to_string(),
                set_path(parent_dir, &recursion_action),
            ));
        }

        let Ok(dir) = std::fs::read_dir(path) else {
            return options;
        };

        for entry in dir {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                options.push(SelectOption::new(
                    format!("📁 {}", path.file_name().unwrap().to_str().unwrap()),
                    set_path(&path, &recursion_action),
                ));
            } else if path.is_file() {
                options.push(SelectOption::new(
                    format!("📄 {}", path.file_name().unwrap().to_str().unwrap()),
                    target_action
                        .clone()
                        .with_argument(Some(ActionArgument::String(
                            path.to_str().unwrap().to_string(),
                        ))),
                ));
            }
        }
        options
    }
}

impl FileChooser {
    pub fn new(context: &UiContext, path: &Path, message: &str, target_action: Action) -> Self {
        static FILE_CHOOSER_UI_ACTION: LazyLock<Action> =
            LazyLock::new(|| Action::new_command("system", "file-chooser-ui"));

        let Action::Command(namespace, name, _) = target_action.clone() else {
            panic!(
                "target_action should be a command action with 3 string arguments (file path, message, original target action namespace, original target action name)"
            );
        };

        let options = file_options(
            path,
            FILE_CHOOSER_UI_ACTION
                .clone()
                .with_argument(Some(ActionArgument::String4(
                    path.to_str().unwrap().to_string(),
                    message.to_string(),
                    namespace.to_string(),
                    name.to_string(),
                ))),
            target_action,
        );

        let select_box =
            SelectBox::new_without_action_name(context, message.to_string(), options, None);
        Self { select_box }
    }
}

impl Model for FileChooser {
    fn set_position(&mut self, position: glam::Vec3) {
        self.select_box.set_position(position);
    }

    fn position(&self) -> glam::Vec3 {
        self.select_box.position()
    }

    fn last_position(&self) -> glam::Vec3 {
        self.select_box.last_position()
    }

    fn focus_position(&self) -> glam::Vec3 {
        self.select_box.focus_position()
    }

    fn set_rotation(&mut self, rotation: glam::Quat) {
        self.select_box.set_rotation(rotation);
    }

    fn rotation(&self) -> glam::Quat {
        self.select_box.rotation()
    }

    fn bound(&self) -> (f32, f32) {
        self.select_box.bound()
    }

    fn glyph_instances(&self) -> Vec<&font_rasterizer::glyph_instances::GlyphInstances> {
        self.select_box.glyph_instances()
    }

    fn vector_instances(&self) -> Vec<&font_rasterizer::vector_instances::VectorInstances<String>> {
        self.select_box.vector_instances()
    }

    fn update(&mut self, context: &UiContext) {
        self.select_box.update(context);
    }

    fn editor_operation(&mut self, op: &text_buffer::action::EditorOperation) {
        self.select_box.editor_operation(op);
    }

    fn model_operation(
        &mut self,
        op: &crate::layout_engine::ModelOperation,
    ) -> crate::layout_engine::ModelOperationResult {
        self.select_box.model_operation(op)
    }

    fn to_string(&self) -> String {
        self.select_box.to_string()
    }

    fn in_animation(&self) -> bool {
        self.select_box.in_animation()
    }

    fn set_border(&mut self, border: crate::layout_engine::ModelBorder) {
        self.select_box.set_border(border);
    }

    fn border(&self) -> crate::layout_engine::ModelBorder {
        self.select_box.border()
    }

    fn set_easing_preset(&mut self, preset: crate::ui_context::CharEasingsPreset) {
        self.select_box.set_easing_preset(preset);
    }
}
