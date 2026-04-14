use std::sync::mpsc::Sender;

use glam::{Quat, Vec3};
use stroke_parser::{Action, ActionArgument};
use text_buffer::action::EditorOperation;

use font_rasterizer::{
    glyph_instances::GlyphInstances, glyph_vertex_buffer::Direction,
    vector_instances::VectorInstances,
};

use crate::ui::StackLayout;
use crate::ui_context::{CharEasingsPreset, UiContext};

use crate::{
    layout_engine::{
        DebugModelDetails, DebugModelNode, DebugTextInputSnapshot, Model, ModelBorder,
    },
    ui_context::{CharEasings, HighlightMode, TextContext},
};

use super::textedit::TextEdit;

const TITLE_TEXT_INDEX: usize = 0;
const INPUT_TEXT_INDEX: usize = 1;

pub struct TextInput {
    action: Action,
    layout: StackLayout,
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
            highlight_mode: HighlightMode::None,
            direction,
            min_bound: (1.0, 1.0).into(),
            ..Default::default()
        }
    }

    pub fn new(
        context: &UiContext,
        message: String,
        default_input: Option<String>,
        action: Action,
    ) -> Self {
        let mut layout = StackLayout::new(context.global_direction());

        let title_text_edit = {
            let mut text_edit = TextEdit::default();
            text_edit.set_config(Self::text_context(context.global_direction()));
            text_edit.editor_operation(&EditorOperation::InsertString(message));

            text_edit
        };
        layout.add_model(Box::new(title_text_edit));
        let input_text_edit = {
            let mut text_edit = TextEdit::default();
            text_edit.set_config(TextContext {
                direction: context.global_direction(),
                min_bound: (1.0, 1.0).into(),
                ..Default::default()
            });
            if let Some(input) = default_input.as_ref() {
                text_edit.editor_operation(&EditorOperation::InsertString(input.to_owned()));
            }
            text_edit
        };
        layout.add_model(Box::new(input_text_edit));
        layout.set_focus_model_index(INPUT_TEXT_INDEX, false);

        Self {
            action,
            layout,
            action_queue_sender: context.action_sender(),
            default_input,
            border: ModelBorder::default(),
        }
    }

    fn title_text_edit(&self) -> &dyn Model {
        self.layout.models()[TITLE_TEXT_INDEX].as_ref()
    }

    fn input_text_edit(&self) -> &dyn Model {
        self.layout.models()[INPUT_TEXT_INDEX].as_ref()
    }

    fn input_text_edit_mut(&mut self) -> &mut dyn Model {
        self.layout.models_mut()[INPUT_TEXT_INDEX].as_mut()
    }
}

impl Model for TextInput {
    fn set_position(&mut self, position: Vec3) {
        self.layout.set_position(position);
    }

    fn position(&self) -> Vec3 {
        self.layout.position()
    }

    fn last_position(&self) -> Vec3 {
        self.layout.last_position()
    }

    fn focus_position(&self) -> Vec3 {
        self.layout.focus_position()
    }

    fn set_rotation(&mut self, rotation: Quat) {
        self.layout.set_rotation(rotation);
    }

    fn rotation(&self) -> Quat {
        self.layout.rotation()
    }

    fn bound(&self) -> (f32, f32) {
        self.layout.bound()
    }

    fn glyph_instances(&self) -> Vec<&GlyphInstances> {
        self.layout.glyph_instances()
    }

    fn vector_instances(&self) -> Vec<&VectorInstances<String>> {
        self.layout.vector_instances()
    }

    fn update(&mut self, context: &UiContext) {
        self.layout.update(context);
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
            | EditorOperation::BackWord => self.input_text_edit_mut().editor_operation(op),
            EditorOperation::InsertEnter => {
                self.action_queue_sender
                    .send(Action::new_command("world", "remove-current"))
                    .unwrap();
                let arg = self.input_text_edit().to_string();

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
        self.layout.model_operation(op)
    }

    fn to_string(&self) -> String {
        [
            self.title_text_edit().to_string(),
            self.input_text_edit().to_string(),
        ]
        .concat()
    }

    fn in_animation(&self) -> bool {
        self.layout.in_animation()
    }

    fn set_border(&mut self, border: ModelBorder) {
        self.border = border;
        self.layout.set_border(border);
    }

    fn border(&self) -> ModelBorder {
        self.border
    }

    fn set_easing_preset(&mut self, preset: CharEasingsPreset) {
        self.layout.set_easing_preset(preset);
    }

    fn debug_node(&self, camera: &crate::camera::Camera) -> DebugModelNode {
        let position = self.position().to_array();
        let last_position = self.last_position().to_array();
        let focus_position = self.focus_position().to_array();
        let rotation = self.rotation().to_array();
        let bound: [f32; 2] = self.bound().into();
        DebugModelNode::new(
            "TextInput",
            self.border(),
            position,
            last_position,
            focus_position,
            rotation,
            bound,
            bound,
            self.in_animation(),
            vec![self.layout.debug_node(camera)],
            DebugModelDetails::TextInput(DebugTextInputSnapshot {
                default_input: self.default_input.clone(),
            }),
            camera,
        )
    }
}
