use std::sync::mpsc::Sender;

use stroke_parser::{Action, CommandName, CommandNamespace};
use text_buffer::action::EditorOperation;

use crate::{
    context::{CharEasings, GpuEasingConfig, StateContext, TextContext},
    instances::GlyphInstances,
    layout_engine::{Model, ModelMode},
};

use super::textedit::TextEdit;

pub struct SelectOption {
    text: String,
    action: Action,
}

impl SelectOption {
    pub fn new(text: String, action: Action) -> Self {
        Self { text, action }
    }
}

pub struct Selectbox {
    selection_offset: usize,
    current_selection: usize,
    options: Vec<SelectOption>,
    text_edit: TextEdit,
    action_queue_sender: Sender<Action>,
}

impl Selectbox {
    pub fn new(
        action_queue_sender: Sender<Action>,
        message: String,
        options: Vec<SelectOption>,
    ) -> Self {
        let config = TextContext {
            max_col: usize::MAX, // SELECTBOX は基本的に改行しないので大きな値を設定
            char_easings: CharEasings {
                select_char: GpuEasingConfig::default(),
                unselect_char: GpuEasingConfig::default(),
                ..Default::default()
            },
            hyde_caret: true,
            ..Default::default()
        };
        let mut text_edit = TextEdit::default();
        text_edit.set_config(config);
        let offset = message.lines().count() + 1;
        text_edit.editor_operation(&EditorOperation::InsertString(message));
        text_edit.editor_operation(&EditorOperation::InsertEnter);
        text_edit.editor_operation(&EditorOperation::InsertEnter);
        text_edit.editor_operation(&EditorOperation::InsertString(
            options
                .iter()
                .map(|s| format!("- {}", s.text))
                .collect::<Vec<String>>()
                .join("\n"),
        ));
        let mut result = Self {
            selection_offset: offset,
            current_selection: 0,
            options,
            text_edit,
            action_queue_sender,
        };
        result.update_current_selection();
        result
    }

    fn update_current_selection(&mut self) {
        self.text_edit.editor_operation(&EditorOperation::UnMark);
        self.text_edit
            .editor_operation(&EditorOperation::BufferHead);
        for _ in 0..(self.selection_offset + self.current_selection) {
            self.text_edit.editor_operation(&EditorOperation::Next);
        }
        self.text_edit.editor_operation(&EditorOperation::Mark);
        self.text_edit.editor_operation(&EditorOperation::Last);
    }
}

impl Model for Selectbox {
    fn set_position(&mut self, position: cgmath::Point3<f32>) {
        self.text_edit.set_position(position);
    }

    fn position(&self) -> cgmath::Point3<f32> {
        self.text_edit.position()
    }

    fn focus_position(&self) -> cgmath::Point3<f32> {
        self.text_edit.focus_position()
    }

    fn set_rotation(&mut self, rotation: cgmath::Quaternion<f32>) {
        self.text_edit.set_rotation(rotation)
    }

    fn rotation(&self) -> cgmath::Quaternion<f32> {
        self.text_edit.rotation()
    }

    fn bound(&self) -> (f32, f32) {
        self.text_edit.bound()
    }

    fn glyph_instances(&self) -> Vec<&GlyphInstances> {
        self.text_edit.glyph_instances()
    }

    fn update(&mut self, context: &StateContext) {
        self.text_edit.update(context)
    }

    fn editor_operation(&mut self, op: &EditorOperation) {
        match op {
            EditorOperation::Previous => {
                self.current_selection =
                    (self.current_selection + self.options.len() - 1) % self.options.len()
            }
            EditorOperation::Next => {
                self.current_selection = (self.current_selection + 1) % self.options.len()
            }
            EditorOperation::BufferHead => self.current_selection = 0,
            EditorOperation::BufferLast => self.current_selection = self.options.len() - 1,
            EditorOperation::InsertEnter => {
                self.action_queue_sender
                    .send(Action::Command(
                        CommandNamespace::new("world".to_string()),
                        CommandName::new("remove-current".to_string()),
                    ))
                    .unwrap();
                self.action_queue_sender
                    .send(self.options[self.current_selection].action.clone())
                    .unwrap();
            }
            // unmark を使っているのがなんか変な気はするなぁ
            EditorOperation::UnMark => {
                self.action_queue_sender
                    .send(Action::Command(
                        CommandNamespace::new("world".to_string()),
                        CommandName::new("remove-current".to_string()),
                    ))
                    .unwrap();
            }
            _ => (),
        }
        self.update_current_selection();
    }

    fn model_operation(
        &mut self,
        op: &crate::layout_engine::ModelOperation,
    ) -> crate::layout_engine::ModelOperationResult {
        // model operation も移譲して問題なさそう
        self.text_edit.model_operation(op)
    }

    fn to_string(&self) -> String {
        self.options
            .iter()
            .map(|s| s.text.clone())
            .collect::<Vec<String>>()
            .join("")
    }

    fn model_mode(&self) -> ModelMode {
        ModelMode::Modal
    }
}
