use std::sync::{Arc, mpsc::Sender};

use log::info;
use similar::{ChangeTag, capture_diff_slices};
use stroke_parser::Action;
use text_buffer::action::EditorOperation;

use font_rasterizer::{
    char_width_calcurator::CharWidthCalculator, context::StateContext,
    glyph_instances::GlyphInstances, glyph_vertex_buffer::Direction,
    vector_instances::VectorInstances,
};

use crate::{
    layout_engine::{Model, ModelBorder},
    ui_context::{CharEasings, GpuEasingConfig, HighlightMode, TextContext},
};

use super::{select_option::SelectOption, textedit::TextEdit};

pub struct SelectBox {
    current_selection: usize,
    options: Vec<SelectOption>,
    title_text_edit: TextEdit,
    search_text_edit: TextEdit,
    select_items_text_edit: TextEdit,
    action_queue_sender: Sender<Action>,
    char_width_calcurator: Arc<CharWidthCalculator>,
    show_action_name: bool,
    cancellable: bool,
    border: ModelBorder,
    max_line: usize,
}

impl SelectBox {
    fn search_context(direction: Direction) -> TextContext {
        TextContext {
            min_bound: (0.0, 0.0).into(),
            direction,
            ..Default::default()
        }
    }

    fn text_context(direction: Direction) -> TextContext {
        TextContext {
            max_col: usize::MAX, // SELECTBOX は基本的に改行しないので大きな値を設定
            char_easings: CharEasings {
                select_char: GpuEasingConfig::default(),
                unselect_char: GpuEasingConfig::default(),
                add_char: GpuEasingConfig::fadein(),
                remove_char: GpuEasingConfig::fadeout(),
                ..Default::default()
            },
            hyde_caret: true,
            highlight_mode: HighlightMode::None,
            min_bound: (0.0, 0.0).into(),
            direction,
            ..Default::default()
        }
    }

    pub fn new(
        context: &StateContext,
        message: String,
        options: Vec<SelectOption>,
        default_narrow: Option<String>,
    ) -> Self {
        Self::inner_new(context, message, options, default_narrow, true, true)
    }

    pub fn new_without_action_name(
        context: &StateContext,
        message: String,
        options: Vec<SelectOption>,
        default_narrow: Option<String>,
    ) -> Self {
        Self::inner_new(context, message, options, default_narrow, false, true)
    }

    pub fn without_cancellable(self) -> Self {
        Self {
            cancellable: false,
            ..self
        }
    }

    fn inner_new(
        context: &StateContext,
        message: String,
        options: Vec<SelectOption>,
        default_narrow: Option<String>,
        show_action_name: bool,
        cancellable: bool,
    ) -> Self {
        let title_text_edit = {
            let mut text_edit = TextEdit::default();
            text_edit.set_config(Self::text_context(context.global_direction));
            text_edit.editor_operation(&EditorOperation::InsertString(message));
            text_edit
        };
        let search_text_edit = {
            let mut text_edit = TextEdit::default();
            text_edit.set_config(Self::search_context(context.global_direction));
            if let Some(default_narrow) = default_narrow {
                text_edit.editor_operation(&EditorOperation::InsertString(format!(
                    "{} ",
                    default_narrow
                )));
            }
            text_edit
        };
        let select_items_text_edit = {
            let mut text_edit = TextEdit::default();
            text_edit.set_config(Self::text_context(context.global_direction));
            text_edit
        };

        let mut result = Self {
            current_selection: 0,
            options,
            title_text_edit,
            search_text_edit,
            select_items_text_edit,
            action_queue_sender: context.action_sender(),
            char_width_calcurator: context.char_width_calcurator.clone(),
            show_action_name,
            cancellable,
            border: ModelBorder::default(),
            max_line: 10,
        };
        result.update_select_items_text_edit();
        result.update_current_selection();
        result.update(context);
        result
    }

    fn narrowd_options(&self) -> Vec<&SelectOption> {
        let text = self.search_text_edit.to_string();
        let search_keywords = text.split_whitespace().collect::<Vec<_>>();
        if search_keywords.is_empty() {
            return self.options.iter().collect::<Vec<_>>();
        }
        self.options
            .iter()
            .filter(|op| {
                if self.show_action_name {
                    op.contains_all(&search_keywords)
                } else {
                    op.contains_all_for_short(&search_keywords)
                }
            })
            .collect::<Vec<_>>()
    }

    fn max_options_len(&self) -> usize {
        self.options
            .iter()
            .map(|opt| self.char_width_calcurator.len(&opt.option_string(0)))
            .max()
            .unwrap_or(0)
    }

    fn max_narrowd_options_len(&self) -> usize {
        self.narrowd_options()
            .iter()
            .map(|opt| self.char_width_calcurator.len(&opt.option_string(0)))
            .max()
            .unwrap_or(0)
    }

    fn update_select_items_text_edit(&mut self) {
        if self.current_selection >= self.narrowd_options().len() {
            self.current_selection = 0;
        }
        let max_options_len = self.max_options_len();
        let current_text: Vec<String> = self
            .select_items_text_edit
            .to_string()
            .lines()
            .map(|s| s.to_owned())
            .collect();
        let next_text: Vec<String> = self
            .narrowd_options()
            .iter()
            .map(|s| {
                if self.show_action_name {
                    // option 毎に文字列をキャッシュするとかもう少し効率のいい方法はあるだろうけど
                    // 今はめんどいのでこれぐらい雑に済ませておく
                    s.option_string(
                        max_options_len
                                    - self.char_width_calcurator.len(&s.option_string(0))
                                    + /* メニューにちょっと余裕を持たせる */2,
                    )
                } else {
                    s.option_string_short()
                }
            })
            .collect();
        let next_text_line_count = next_text.len();
        let next_text = if next_text_line_count > self.max_line {
            // current_selection が max_line より小さい場合は先頭から max_line 行分
            // current_selection が最後から max_line より小さい場合は最後から max_line 行分
            // current_selection を中心に max_line 行分だけ表示する
            let start = if self.current_selection < self.max_line / 2 {
                0
            } else if next_text_line_count - self.current_selection <= self.max_line / 2 {
                next_text_line_count - self.max_line
            } else {
                self.current_selection - (self.max_line / 2)
            };
            let end = (start + self.max_line).min(next_text_line_count);
            next_text[start..end].to_vec()
        } else {
            next_text
        };

        let diff = capture_diff_slices(similar::Algorithm::Patience, &current_text, &next_text);

        // まずはバッファの先頭に移動しておく
        self.select_items_text_edit
            .editor_operation(&EditorOperation::BufferHead);
        info!("------------");
        for change in diff
            .iter()
            .flat_map(|d| d.iter_changes(&current_text, &next_text))
        {
            info!("change: {:?}", change);
            match change.tag() {
                ChangeTag::Equal => {
                    // 同じ場合は次の行に移動する
                    self.select_items_text_edit
                        .editor_operation(&EditorOperation::Next);
                }
                ChangeTag::Insert => {
                    // 追加の場合はそのまま挿入する
                    self.select_items_text_edit
                        .editor_operation(&EditorOperation::InsertString(
                            change.value().trim().to_owned(),
                        ));
                    self.select_items_text_edit
                        .editor_operation(&EditorOperation::InsertEnter);
                }
                ChangeTag::Delete => {
                    // 削除の場合は削除する
                    self.select_items_text_edit
                        .editor_operation(&EditorOperation::Mark);
                    self.select_items_text_edit
                        .editor_operation(&EditorOperation::Last);
                    self.select_items_text_edit
                        .editor_operation(&EditorOperation::Cut(|_| {}));
                    self.select_items_text_edit
                        .editor_operation(&EditorOperation::Delete);
                }
            }
        }
        self.select_items_text_edit
            .editor_operation(&EditorOperation::BufferHead);
    }

    // 現在の選択肢数と max_line の関係を考慮してオフセットを計算する
    fn selection_offset(&self) -> usize {
        let option_len = self.narrowd_options().len();
        // max_line が選択肢数より大きい場合は current_selection をそのまま返す
        if option_len <= self.max_line {
            return self.current_selection;
        }
        if self.current_selection <= self.max_line / 2 {
            return self.current_selection;
        }
        if self.current_selection + self.max_line / 2 >= option_len {
            return self.current_selection - (option_len - self.max_line);
        }
        self.max_line / 2
    }

    fn update_current_selection(&mut self) {
        self.select_items_text_edit
            .editor_operation(&EditorOperation::UnMark);
        self.select_items_text_edit
            .editor_operation(&EditorOperation::BufferHead);
        for _ in 0..(self.selection_offset()) {
            self.select_items_text_edit
                .editor_operation(&EditorOperation::Next);
        }
        self.select_items_text_edit
            .editor_operation(&EditorOperation::Mark);
        self.select_items_text_edit
            .editor_operation(&EditorOperation::Last);
    }
}

impl Model for SelectBox {
    fn set_position(&mut self, position: cgmath::Point3<f32>) {
        let (bound_width, bound_height) = self.select_items_text_edit.bound();

        let (title_offset, search_offset) = match self.select_items_text_edit.direction() {
            Direction::Horizontal => (
                cgmath::Vector3::new(0.0, -(2.0 + bound_height / 2.0), 0.0),
                cgmath::Vector3::new(0.0, -(1.0 + bound_height / 2.0), 0.0),
            ),
            Direction::Vertical => (
                cgmath::Vector3::new(-(2.0 + bound_width / 2.0), 0.0, 0.0),
                cgmath::Vector3::new(-(1.0 + bound_width / 2.0), 0.0, 0.0),
            ),
        };
        self.title_text_edit.set_position(position - title_offset);
        self.search_text_edit.set_position(position - search_offset);
        self.select_items_text_edit.set_position(position);
    }

    fn position(&self) -> cgmath::Point3<f32> {
        self.select_items_text_edit.position()
    }

    fn last_position(&self) -> cgmath::Point3<f32> {
        self.select_items_text_edit.last_position()
    }

    fn focus_position(&self) -> cgmath::Point3<f32> {
        // TODO last の値を取ってくる必要がある
        let (x, y, z) = self.title_text_edit.last_position().into();
        let (bound_width, bound_height) = self.select_items_text_edit.bound();
        match self.select_items_text_edit.direction() {
            Direction::Horizontal => cgmath::Point3::new(x, y - bound_height / 2.0, z),
            Direction::Vertical => cgmath::Point3::new(x - bound_width / 2.0, y, z),
        }
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
        let bounds = [
            self.title_text_edit.bound(),
            self.search_text_edit.bound(),
            self.select_items_text_edit.bound(),
        ];
        match self.select_items_text_edit.direction() {
            Direction::Horizontal => (
                bounds
                    .iter()
                    .map(|(width, _)| *width)
                    .fold(f32::NAN, f32::max),
                bounds.iter().map(|(_, height)| height).sum::<f32>(),
            ),
            Direction::Vertical => (
                bounds.iter().map(|(width, _)| width).sum::<f32>(),
                bounds
                    .iter()
                    .map(|(_, height)| *height)
                    .fold(f32::NAN, f32::max),
            ),
        }
    }

    fn glyph_instances(&self) -> Vec<&GlyphInstances> {
        [
            self.title_text_edit.glyph_instances(),
            self.search_text_edit.glyph_instances(),
            self.select_items_text_edit.glyph_instances(),
        ]
        .concat()
    }

    fn vector_instances(&self) -> Vec<&VectorInstances<String>> {
        [
            self.title_text_edit.vector_instances(),
            self.search_text_edit.vector_instances(),
            self.select_items_text_edit.vector_instances(),
        ]
        .concat()
    }

    fn update(&mut self, context: &StateContext) {
        self.title_text_edit.update(context);
        self.search_text_edit.update(context);
        self.select_items_text_edit.update(context);
    }

    fn editor_operation(&mut self, op: &EditorOperation) {
        let narrowed_options_len = self.narrowd_options().len();
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
                if self.max_narrowd_options_len() == 0 {
                    return;
                }
                self.current_selection =
                    (self.current_selection + narrowed_options_len - 1) % narrowed_options_len;
                self.update_select_items_text_edit()
            }
            EditorOperation::Next => {
                if self.max_narrowd_options_len() == 0 {
                    return;
                }
                self.current_selection = (self.current_selection + 1) % narrowed_options_len;
                self.update_select_items_text_edit()
            }
            EditorOperation::BufferHead => {
                self.current_selection = 0;
                self.update_select_items_text_edit();
            }
            EditorOperation::BufferLast => {
                self.current_selection = narrowed_options_len - 1;
                self.update_select_items_text_edit();
            }
            EditorOperation::InsertEnter => {
                if let Some(option) = self.narrowd_options().get(self.current_selection) {
                    self.action_queue_sender
                        .send(Action::new_command("world", "remove-current"))
                        .unwrap();
                    self.action_queue_sender
                        .send(Action::new_command("world", "fit-by-direction"))
                        .unwrap();
                    option
                        .actions
                        .iter()
                        .for_each(|action| self.action_queue_sender.send(action.clone()).unwrap())
                }
                // Option がない場合は何もしない
                // TODO 何かしらのエラーメッセージを出すべきか？
            }
            // unmark を使っているのがなんか変な気はするなぁ
            EditorOperation::UnMark => {
                if self.cancellable {
                    self.action_queue_sender
                        .send(Action::new_command("world", "remove-current"))
                        .unwrap();
                    self.action_queue_sender
                        .send(Action::new_command("world", "fit-by-direction"))
                        .unwrap();
                }
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
        // 返り値は適当に select_items_text_edit のものだけ返せばよさそう
        self.title_text_edit.model_operation(op);
        self.search_text_edit.model_operation(op);
        self.select_items_text_edit.model_operation(op)
    }

    fn to_string(&self) -> String {
        [
            self.title_text_edit.to_string(),
            self.search_text_edit.to_string(),
            // options は select_items_text_edit の to_string だけだと最初に絞り込まれていない
            // 選択肢の文字列が出てこないので全選択肢の文字列を出力する
            self.options
                .iter()
                .map(|s| {
                    if self.show_action_name {
                        s.option_string(0)
                    } else {
                        s.option_string_short()
                    }
                })
                .collect::<Vec<String>>()
                .join(""),
        ]
        .concat()
    }

    fn in_animation(&self) -> bool {
        self.title_text_edit.in_animation()
            || self.search_text_edit.in_animation()
            || self.select_items_text_edit.in_animation()
    }

    fn set_border(&mut self, border: ModelBorder) {
        self.border = border;
    }

    fn border(&self) -> ModelBorder {
        self.border
    }
}
