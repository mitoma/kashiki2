use std::sync::mpsc::Sender;

use stroke_parser::{Action, ActionArgument};
use text_buffer::action::EditorOperation;

use font_rasterizer::{
    context::StateContext, glyph_instances::GlyphInstances, glyph_vertex_buffer::Direction,
};

use crate::{
    layout_engine::{Model, ModelBorder, ModelMode},
    ui_context::{CharEasings, TextContext},
};

use super::textedit::TextEdit;

pub struct TextInput {
    action: Action,
    title_text_edit: TextEdit,
    input_text_edit: TextEdit,
    action_queue_sender: Sender<Action>,
    default_input: Option<String>,
    border: ModelBorder,
}

impl TextInput {
    fn text_context(direction: Direction) -> TextContext {
        TextContext {
            max_col: usize::MAX,
            char_easings: CharEasings::zero_motion(),
            hyde_caret: true,
            direction,
            ..Default::default()
        }
    }

    pub fn new(
        context: &StateContext,
        message: String,
        default_input: Option<String>,
        action: Action,
    ) -> Self {
        let title_text_edit = {
            let mut text_edit = TextEdit::default();
            text_edit.set_config(Self::text_context(context.global_direction));
            text_edit.editor_operation(&EditorOperation::InsertString(message));

            text_edit
        };
        let input_text_edit = {
            let mut text_edit = TextEdit::default();
            if let Some(input) = default_input.as_ref() {
                text_edit.editor_operation(&EditorOperation::InsertString(input.to_owned()));
            }
            text_edit
        };

        Self {
            action,
            title_text_edit,
            input_text_edit,
            action_queue_sender: context.action_queue_sender.clone(),
            default_input,
            border: ModelBorder::default(),
        }
    }
}

impl Model for TextInput {
    fn set_position(&mut self, position: cgmath::Point3<f32>) {
        let title_offset = match self.input_text_edit.direction() {
            Direction::Horizontal => cgmath::Vector3::new(0.0, -1.0, 0.0),
            Direction::Vertical => cgmath::Vector3::new(-1.0, 0.0, 0.0),
        };
        self.title_text_edit.set_position(position - title_offset);
        self.input_text_edit.set_position(position);
    }

    fn position(&self) -> cgmath::Point3<f32> {
        self.input_text_edit.position()
    }

    fn focus_position(&self) -> cgmath::Point3<f32> {
        self.input_text_edit.focus_position()
    }

    fn set_rotation(&mut self, rotation: cgmath::Quaternion<f32>) {
        self.title_text_edit.set_rotation(rotation);
        self.input_text_edit.set_rotation(rotation);
    }

    fn rotation(&self) -> cgmath::Quaternion<f32> {
        self.input_text_edit.rotation()
    }

    fn bound(&self) -> (f32, f32) {
        self.input_text_edit.bound()
    }

    fn glyph_instances(&self) -> Vec<&GlyphInstances> {
        [
            self.title_text_edit.glyph_instances(),
            self.input_text_edit.glyph_instances(),
        ]
        .concat()
    }

    fn update(&mut self, context: &StateContext) {
        self.title_text_edit.update(context);
        self.input_text_edit.update(context);
    }

    fn editor_operation(&mut self, op: &EditorOperation) {
        match op {
            EditorOperation::InsertChar(_)
            | EditorOperation::InsertString(_)
            | EditorOperation::Backspace
            | EditorOperation::BackspaceWord
            | EditorOperation::Delete
            | EditorOperation::DeleteWord
            | EditorOperation::Head
            | EditorOperation::Last
            | EditorOperation::Forward
            | EditorOperation::ForwardWord
            | EditorOperation::Back
            | EditorOperation::BackWord => self.input_text_edit.editor_operation(op),
            EditorOperation::InsertEnter => {
                self.action_queue_sender
                    .send(Action::new_command("world", "remove-current"))
                    .unwrap();
                let arg = self.input_text_edit.to_string();

                let action = match &self.action {
                    Action::Command(namespace, name, _arg) => Action::Command(
                        namespace.clone(),
                        name.clone(),
                        ActionArgument::String2(
                            arg,
                            // default value が指定されていた時にはそれを第二引数として渡す
                            self.default_input.clone().unwrap_or_default(),
                        ),
                    ),
                    Action::Keytype(_)
                    | Action::ImeEnable
                    | Action::ImeDisable
                    | Action::ImePreedit(_, _)
                    | Action::ImeInput(_) => self.action.clone(),
                };
                self.action_queue_sender.send(action).unwrap();
            }
            // unmark を使っているのがなんか変な気はするなぁ
            EditorOperation::UnMark => {
                self.action_queue_sender
                    .send(Action::new_command("world", "remove-current"))
                    .unwrap();
            }
            _ => (),
        }
    }

    fn model_operation(
        &mut self,
        op: &crate::layout_engine::ModelOperation,
    ) -> crate::layout_engine::ModelOperationResult {
        // model operation も移譲して問題なさそう
        // 返り値は適当に input_text_edit のものだけ返せばよさそう
        self.title_text_edit.model_operation(op);
        self.input_text_edit.model_operation(op)
    }

    fn to_string(&self) -> String {
        [
            self.title_text_edit.to_string(),
            self.input_text_edit.to_string(),
        ]
        .concat()
    }

    fn model_mode(&self) -> ModelMode {
        ModelMode::Modal
    }

    fn in_animation(&self) -> bool {
        self.title_text_edit.in_animation() || self.input_text_edit.in_animation()
    }

    fn set_border(&mut self, border: ModelBorder) {
        self.border = border;
    }

    fn border(&self) -> ModelBorder {
        self.border
    }
}
