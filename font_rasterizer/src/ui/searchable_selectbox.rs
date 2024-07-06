use std::sync::mpsc::Sender;

use stroke_parser::Action;
use strsim::levenshtein;
use text_buffer::action::EditorOperation;

use crate::{
    context::{
        CharEasings, CpuEasingConfig, GpuEasingConfig, RemoveCharMode, StateContext, TextContext,
    },
    instances::GlyphInstances,
    layout_engine::{Model, ModelMode},
};

use super::{selectbox::SelectOption, textedit::TextEdit};

pub struct SearchableSelectBox {
    current_selection: usize,
    options: Vec<SelectOption>,
    title_text_edit: TextEdit,
    search_text_edit: TextEdit,
    select_items_text_edit: TextEdit,
    action_queue_sender: Sender<Action>,
}

impl SearchableSelectBox {
    fn text_context() -> TextContext {
        TextContext {
            max_col: usize::MAX, // SELECTBOX は基本的に改行しないので大きな値を設定
            char_easings: CharEasings {
                select_char: GpuEasingConfig::default(),
                unselect_char: GpuEasingConfig::default(),
                add_char: GpuEasingConfig::default(),
                remove_char: GpuEasingConfig::default(),
                remove_char_mode: RemoveCharMode::Immediate,
                position_easing: CpuEasingConfig::zero_motion(),
                ..Default::default()
            },
            hyde_caret: true,
            ..Default::default()
        }
    }

    pub fn new(
        action_queue_sender: Sender<Action>,
        message: String,
        options: Vec<SelectOption>,
    ) -> Self {
        let title_text_edit = {
            let mut text_edit = TextEdit::default();
            text_edit.set_config(Self::text_context());
            text_edit.editor_operation(&EditorOperation::InsertString(message));

            text_edit
        };
        let search_text_edit = TextEdit::default();
        let select_items_text_edit = {
            let mut text_edit = TextEdit::default();
            text_edit.set_config(Self::text_context());
            text_edit
        };

        let mut result = Self {
            current_selection: 0,
            options,
            title_text_edit,
            search_text_edit,
            select_items_text_edit,
            action_queue_sender,
        };
        result.update_select_items_text_edit();
        result.update_current_selection();
        result
    }

    fn clear_text_edit(text_edit: &mut TextEdit) {
        text_edit.editor_operation(&EditorOperation::BufferHead);
        text_edit.editor_operation(&EditorOperation::Mark);
        text_edit.editor_operation(&EditorOperation::BufferLast);
        text_edit.editor_operation(&EditorOperation::Cut(|_| {}));
    }

    // レーベンシュタイン距離が検索文字に比較して小さいものを候補として返す
    fn narrowd_options(&self) -> Vec<&SelectOption> {
        let search_keywords = self.search_text_edit.to_string().trim().to_owned();
        if search_keywords.is_empty() {
            return self.options.iter().collect::<Vec<_>>();
        }
        let mut result = self.options.iter().collect::<Vec<_>>();
        result.sort_by(|l, r| {
            levenshtein(&search_keywords, &l.option_string())
                .cmp(&levenshtein(&search_keywords, &r.option_string()))
        });
        result
    }

    fn update_select_items_text_edit(&mut self) {
        Self::clear_text_edit(&mut self.select_items_text_edit);
        self.select_items_text_edit
            .editor_operation(&EditorOperation::InsertString(
                self.narrowd_options()
                    .iter()
                    .map(|s| format!("- {}", s.option_string()))
                    .collect::<Vec<String>>()
                    .join("\n"),
            ));
    }

    fn update_current_selection(&mut self) {
        self.select_items_text_edit
            .editor_operation(&EditorOperation::UnMark);
        self.select_items_text_edit
            .editor_operation(&EditorOperation::BufferHead);
        for _ in 0..(self.current_selection) {
            self.select_items_text_edit
                .editor_operation(&EditorOperation::Next);
        }
        self.select_items_text_edit
            .editor_operation(&EditorOperation::Mark);
        self.select_items_text_edit
            .editor_operation(&EditorOperation::Last);
    }
}

impl Model for SearchableSelectBox {
    fn set_position(&mut self, position: cgmath::Point3<f32>) {
        self.title_text_edit
            .set_position(position - cgmath::Vector3::new(0.0, -2.0, 0.0));
        self.search_text_edit
            .set_position(position - cgmath::Vector3::new(0.0, -1.0, 0.0));
        self.select_items_text_edit.set_position(position);
    }

    fn position(&self) -> cgmath::Point3<f32> {
        self.select_items_text_edit.position()
    }

    fn focus_position(&self) -> cgmath::Point3<f32> {
        self.select_items_text_edit.focus_position()
    }

    fn set_rotation(&mut self, rotation: cgmath::Quaternion<f32>) {
        self.title_text_edit.set_rotation(rotation);
        self.search_text_edit.set_rotation(rotation);
        self.select_items_text_edit.set_rotation(rotation)
    }

    fn rotation(&self) -> cgmath::Quaternion<f32> {
        self.select_items_text_edit.rotation()
    }

    fn bound(&self) -> (f32, f32) {
        self.select_items_text_edit.bound()
    }

    fn glyph_instances(&self) -> Vec<&GlyphInstances> {
        [
            self.title_text_edit.glyph_instances(),
            self.search_text_edit.glyph_instances(),
            self.select_items_text_edit.glyph_instances(),
        ]
        .concat()
    }

    fn update(&mut self, context: &StateContext) {
        self.title_text_edit.update(context);
        self.search_text_edit.update(context);
        self.select_items_text_edit.update(context)
    }

    fn editor_operation(&mut self, op: &EditorOperation) {
        match op {
            EditorOperation::InsertChar(_)
            | EditorOperation::InsertString(_)
            | EditorOperation::Backspace
            | EditorOperation::BackspaceWord
            | EditorOperation::Delete
            | EditorOperation::DeleteWord => {
                self.search_text_edit.editor_operation(op);
                self.update_select_items_text_edit();
            }
            EditorOperation::Head
            | EditorOperation::Last
            | EditorOperation::Forward
            | EditorOperation::ForwardWord
            | EditorOperation::Back
            | EditorOperation::BackWord => self.search_text_edit.editor_operation(op),
            // search_items_text_edit に対して操作を行う
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
                    .send(Action::new_command("world", "remove-current"))
                    .unwrap();
                self.narrowd_options()[self.current_selection]
                    .actions
                    .iter()
                    .for_each(|action| self.action_queue_sender.send(action.clone()).unwrap());
            }
            // unmark を使っているのがなんか変な気はするなぁ
            EditorOperation::UnMark => {
                self.action_queue_sender
                    .send(Action::new_command("world", "remove-current"))
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
        self.select_items_text_edit.model_operation(op)
    }

    fn to_string(&self) -> String {
        [
            self.title_text_edit.to_string(),
            self.search_text_edit.to_string(),
            self.select_items_text_edit.to_string(),
        ]
        .concat()
        /*
        self.options
            .iter()
            .map(|s| s.text.clone())
            .collect::<Vec<String>>()
            .join("")
             */
    }

    fn model_mode(&self) -> ModelMode {
        ModelMode::Modal
    }
}
