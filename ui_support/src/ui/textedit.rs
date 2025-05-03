use std::{
    ops::Range,
    sync::{
        Arc,
        mpsc::{Receiver, Sender},
    },
};

use cgmath::{Point3, Quaternion, Rotation3};

use text_buffer::{
    action::EditorOperation,
    buffer::CellPosition,
    caret::{Caret, CaretType},
    editor::{ChangeEvent, CharWidthResolver, Editor, PhisicalLayout},
};

use font_rasterizer::{
    char_width_calcurator::{CharWidth, CharWidthCalculator},
    color_theme::{ColorTheme, ThemedColor},
    context::StateContext,
    glyph_instances::GlyphInstances,
    glyph_vertex_buffer::Direction,
    vector_instances::VectorInstances,
};

use crate::{
    easing_value::EasingPointN,
    layout_engine::{
        Model, ModelAttributes, ModelBorder, ModelMode, ModelOperation, ModelOperationResult,
    },
    ui_context::TextContext,
};

use super::{
    caret_char,
    view_element_state::{CaretStates, CharStates, ViewElementStateUpdateRequest},
};

pub struct TextEdit {
    config: TextContext,

    editor: Editor,
    receiver: Receiver<ChangeEvent>,

    text_edit_operation_sender: Sender<TextEditOperation>,
    text_edit_operation_receiver: Receiver<TextEditOperation>,

    char_states: CharStates,
    caret_states: CaretStates,

    text_updated: bool,
    config_updated: bool,

    position: EasingPointN<3>,
    rotation: EasingPointN<4>,
    world_scale: [f32; 2],

    bound: EasingPointN<2>,

    border: ModelBorder,
}

impl Default for TextEdit {
    fn default() -> Self {
        let config = TextContext::default();
        let (tx, rx) = std::sync::mpsc::channel();
        let (text_edit_operation_sender, text_edit_operation_receiver) = std::sync::mpsc::channel();

        let position = EasingPointN::new([0.0, 0.0, 0.0]);
        let bound = config.min_bound.into();
        let rotation =
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_y(), cgmath::Deg(0.0));
        let [x, y, z] = rotation.v.into();
        let s = rotation.s;
        let rotation = EasingPointN::new([x, y, z, s]);
        Self {
            config,
            editor: Editor::new(tx),
            receiver: rx,

            text_edit_operation_sender,
            text_edit_operation_receiver,

            char_states: CharStates::default(),
            caret_states: CaretStates::default(),

            text_updated: true,
            config_updated: true,

            position,
            rotation,
            world_scale: [1.0, 1.0],
            bound,
            border: ModelBorder::default(),
        }
    }
}

impl TextEdit {
    pub fn new(config: TextContext) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }
}

impl Model for TextEdit {
    fn set_position(&mut self, position: Point3<f32>) {
        let p: [f32; 3] = position.into();
        if self.position.last() == p {
            return;
        }
        self.position.update(position.into());
    }

    fn position(&self) -> cgmath::Point3<f32> {
        self.position.current().into()
    }

    // キャレットの位置と direction を考慮してテキストエディタ中のフォーカス位置を返す
    fn focus_position(&self) -> Point3<f32> {
        let [caret_position_x, caret_position_y, _caret_position_z] = self
            .caret_states
            .main_caret_position()
            .unwrap_or([0.0, 0.0, 0.0]);

        let [position_x, position_y, position_z] = self.position.last();
        let [current_bound_x, current_bound_y] = self.bound.last();
        match self.config.direction {
            Direction::Horizontal => Point3::new(
                position_x,
                position_y + caret_position_y + current_bound_y / 2.0,
                position_z,
            ),
            Direction::Vertical => Point3::new(
                position_x + caret_position_x - current_bound_x / 2.0,
                position_y,
                position_z,
            ),
        }
    }

    fn set_rotation(&mut self, rotation: Quaternion<f32>) {
        let [x, y, z] = rotation.v.into();
        let s = rotation.s;
        if self.rotation.last() == [x, y, z, s] {
            return;
        }
        self.rotation.update([x, y, z, s]);
        self.config_updated = true;
    }

    fn rotation(&self) -> Quaternion<f32> {
        self.rotation.last().into()
    }

    fn bound(&self) -> (f32, f32) {
        // 外向けにはアニメーション完了後の最終的なサイズを返す
        // この値はレイアウトの計算に使われるためである
        self.bound.last().into()
    }

    fn glyph_instances(&self) -> Vec<&GlyphInstances> {
        self.char_states.instances.to_instances()
    }

    fn vector_instances(&self) -> Vec<&VectorInstances<String>> {
        if self.config.hyde_caret {
            vec![]
        } else {
            self.caret_states.instances.to_instances()
        }
    }

    fn update(&mut self, context: &StateContext) {
        let color_theme = &context.color_theme;
        let device = &context.device;
        let queue = &context.queue;
        if self.config.color_theme != *color_theme {
            self.config.color_theme = *color_theme;
            self.char_states.update_char_theme(color_theme);
            self.config_updated = true;
        }

        self.sync_editor_events(device, color_theme);

        if self.text_updated || self.config_updated {
            let layout = self.calc_phisical_layout(context.char_width_calcurator.clone());
            let bound = self.calc_bound(&layout);
            self.calc_position(&context.char_width_calcurator, &layout, bound);
        }

        self.calc_instance_positions(&context.char_width_calcurator);
        self.char_states.instances.update(device, queue);
        self.caret_states.instances.update(device, queue);

        self.text_updated = false;
        self.config_updated = false;
    }

    fn editor_operation(&mut self, op: &text_buffer::action::EditorOperation) {
        self.editor.operation(op)
    }

    fn model_operation(&mut self, op: &ModelOperation) -> ModelOperationResult {
        match op {
            ModelOperation::ChangeDirection(direction) => {
                self.config.direction = if let Some(direction) = direction {
                    *direction
                } else {
                    self.config.direction.toggle()
                };
                self.char_states
                    .instances
                    .set_direction(&self.config.direction);
                self.text_updated = true;
                ModelOperationResult::RequireReLayout
            }
            ModelOperation::IncreaseRowInterval => {
                self.config.row_interval += 0.05;
                self.text_updated = true;
                ModelOperationResult::RequireReLayout
            }
            ModelOperation::DecreaseRowInterval => {
                self.config.row_interval -= 0.05;
                self.text_updated = true;
                ModelOperationResult::RequireReLayout
            }
            ModelOperation::IncreaseColInterval => {
                self.config.col_interval += 0.05;
                self.text_updated = true;
                ModelOperationResult::RequireReLayout
            }
            ModelOperation::DecreaseColInterval => {
                self.config.col_interval -= 0.05;
                self.text_updated = true;
                ModelOperationResult::RequireReLayout
            }
            ModelOperation::IncreaseRowScale => {
                self.config.row_scale += 0.05;
                self.text_updated = true;
                ModelOperationResult::RequireReLayout
            }
            ModelOperation::DecreaseRowScale => {
                self.config.row_scale -= 0.05;
                self.text_updated = true;
                ModelOperationResult::RequireReLayout
            }
            ModelOperation::IncreaseColScale => {
                self.config.col_scale += 0.05;
                self.text_updated = true;
                ModelOperationResult::RequireReLayout
            }
            ModelOperation::DecreaseColScale => {
                self.config.col_scale -= 0.05;
                self.text_updated = true;
                ModelOperationResult::RequireReLayout
            }
            ModelOperation::CopyDisplayString(width_resolver, result_callback) => {
                result_callback(
                    self.editor
                        .calc_phisical_layout(
                            self.max_display_width(),
                            &self.config.line_prohibited_chars,
                            width_resolver.clone(),
                        )
                        .to_string(),
                );
                ModelOperationResult::NoCare
            }
            ModelOperation::TogglePsychedelic => {
                self.config.psychedelic = !self.config.psychedelic;
                self.char_states.set_motion_and_color(&self.config);
                ModelOperationResult::RequireReLayout
            }
            ModelOperation::MoveToClick(x, y, view_projection_matrix) => {
                if let Some(buffer_char) = self.char_states.get_nearest_char(
                    *x,
                    *y,
                    view_projection_matrix,
                    &self.model_attributes(),
                ) {
                    self.editor_operation(&EditorOperation::MoveTo(Caret::new_without_event(
                        buffer_char.position,
                        CaretType::Primary,
                    )));
                    self.char_states.notify_char(buffer_char, &self.config);
                }
                ModelOperationResult::RequireReLayout
            }
            ModelOperation::MarkAndClick(_, _, _) => todo!(),
            ModelOperation::ToggleMinBound => {
                self.config.toggle_min_bound();
                self.config_updated = true;
                ModelOperationResult::RequireReLayout
            }
        }
    }

    fn to_string(&self) -> String {
        self.editor.to_buffer_string()
    }

    fn model_mode(&self) -> ModelMode {
        ModelMode::Nomal
    }

    fn in_animation(&self) -> bool {
        self.position.in_animation() || self.bound.in_animation() || self.rotation.in_animation()
    }

    fn set_border(&mut self, border: ModelBorder) {
        self.border = border;
    }

    fn border(&self) -> ModelBorder {
        self.border
    }
}

impl TextEdit {
    pub(crate) fn text_edit_operation(&mut self, op: TextEditOperation) {
        self.text_edit_operation_sender.send(op).unwrap();
    }

    // editor から受け取ったイベントを TextEdit の caret, buffer_chars, instances に同期する。
    #[inline]
    fn sync_editor_events(&mut self, device: &wgpu::Device, color_theme: &ColorTheme) {
        #[derive(Default)]
        struct CharChangeCounter {
            add_char: u32,
            move_char: u32,
            remove_char: u32,
        }

        let mut char_change_counter = CharChangeCounter::default();
        while let Ok(event) = self.receiver.try_recv() {
            self.text_updated = true;
            match event {
                ChangeEvent::AddChar(c) => {
                    let caret_pos = self
                        .caret_states
                        .main_caret_position()
                        .unwrap_or([0.0, 1.0, 0.0]);
                    self.char_states.add_char(
                        c,
                        caret_pos,
                        color_theme.text().get_color(),
                        char_change_counter.add_char,
                        &self.config,
                        device,
                    );
                    char_change_counter.add_char += 1;
                }
                ChangeEvent::MoveChar { from, to } => {
                    if let Some([row, _col]) = self.caret_states.main_caret_logical_position() {
                        if from.position.row == row || to.position.row == row {
                            self.char_states.move_char(
                                from,
                                to,
                                char_change_counter.move_char,
                                &self.config,
                                device,
                            );
                            char_change_counter.move_char += 1;
                        } else {
                            self.char_states
                                .move_char(from, to, 0, &self.config, device);
                        }
                    }
                    self.char_states
                        .move_char(from, to, 0, &self.config, device);
                }
                ChangeEvent::RemoveChar(c) => {
                    self.char_states.char_to_dustbox(
                        c,
                        char_change_counter.remove_char,
                        &self.config,
                    );
                    char_change_counter.remove_char += 1;
                }
                ChangeEvent::SelectChar(c) => self.char_states.select_char(c, &self.config),
                ChangeEvent::UnSelectChar(c) => self.char_states.unselect_char(c, &self.config),
                ChangeEvent::AddCaret(c) => {
                    self.caret_states.add_caret(
                        c,
                        color_theme.text_emphasized().get_color(),
                        device,
                    );
                }
                ChangeEvent::MoveCaret { from, to } => {
                    self.caret_states.move_caret(from, to, device);
                }
                ChangeEvent::RemoveCaret(c) => {
                    self.caret_states.caret_to_dustbox(c);
                }
            }
        }
        // editor のイベントを処理した後に textedit 特有の Operation を処理する
        // そうしなければ char_state のインスタンスが期待通りに存在しないため
        while let Ok(event) = self.text_edit_operation_receiver.try_recv() {
            match event {
                TextEditOperation::SetThemedColor(range, color) => {
                    self.char_states.update_states(
                        &range,
                        &ViewElementStateUpdateRequest {
                            base_color: Some(color),
                            ..Default::default()
                        },
                        &self.config,
                    );
                }
            }
        }
    }

    #[inline]
    fn calc_phisical_layout(
        &mut self,
        char_width_calcurator: Arc<dyn CharWidthResolver>,
    ) -> PhisicalLayout {
        self.editor.calc_phisical_layout(
            self.max_display_width(),
            &self.config.line_prohibited_chars,
            char_width_calcurator,
        )
    }

    // レイアウト情報から bound の計算を行い更新する
    #[inline]
    fn calc_bound(&mut self, layout: &PhisicalLayout) -> [f32; 2] {
        // update bound
        let (max_col, max_row) = layout.chars.iter().fold((0, 0), |result, (_, pos)| {
            (result.0.max(pos.col), result.1.max(pos.row))
        });
        let [max_x, max_y, _max_z] = Self::get_adjusted_position(
            &self.config,
            CharWidth::Wide, /* この指定に深い意図はない */
            [0.0, 0.0],      /* bound の計算時には考慮不要なのでゼロのベクトルを渡す */
            [max_col, max_row],
        );
        let (max_x, max_y) = (
            max_x.abs().max(self.config.min_bound.x),
            max_y.abs().max(self.config.min_bound.y),
        );
        let bound = (max_x.abs(), max_y.abs()).into();
        self.bound.update(bound);
        bound
    }

    // 文字と caret の model 上の x, y の位置を計算
    #[inline]
    fn calc_position(
        &mut self,
        char_width_calcurator: &CharWidthCalculator,
        layout: &PhisicalLayout,
        bound: [f32; 2],
    ) {
        // update char position
        layout.chars.iter().for_each(|(c, pos)| {
            let width = char_width_calcurator.get_width(c.c);
            let position =
                Self::get_adjusted_position(&self.config, width, bound, [pos.col, pos.row]);
            self.char_states.update_state(
                c,
                &ViewElementStateUpdateRequest {
                    position: Some(position),
                    scale: Some(self.config.instance_scale()),
                    ..Default::default()
                },
                &self.config,
            )
        });

        // update caret position
        {
            let caret_width = char_width_calcurator.get_width(caret_char(CaretType::Primary));
            let position = Self::get_adjusted_position(
                &self.config,
                caret_width,
                bound,
                [layout.main_caret_pos.col, layout.main_caret_pos.row],
            );
            self.caret_states.update_state_position_and_scale(
                CaretType::Primary,
                position,
                self.config.instance_scale(),
            );
        }
        if let Some(mark_pos) = layout.mark_pos {
            let caret_width = char_width_calcurator.get_width(caret_char(CaretType::Mark));
            let position = Self::get_adjusted_position(
                &self.config,
                caret_width,
                bound,
                [mark_pos.col, mark_pos.row],
            );
            self.caret_states.update_state_position_and_scale(
                CaretType::Mark,
                position,
                self.config.instance_scale(),
            );
        }
    }

    #[inline]
    fn get_adjusted_position(
        config: &TextContext,
        char_width: CharWidth,
        [bound_x, _bound_y]: [f32; 2],
        [x, y]: [usize; 2],
    ) -> [f32; 3] {
        let x = ((x as f32) / 2.0 + char_width.left()) * config.col_interval;
        let y = y as f32 * config.row_interval;
        match config.direction {
            Direction::Horizontal => [x, -y, 0.0],
            Direction::Vertical => [bound_x - y, -x, 0.0],
        }
    }

    #[inline]
    fn model_attributes(&self) -> ModelAttributes {
        let [bound_x, bound_y] = &self.bound.current();
        let center = (bound_x / 2.0, -bound_y / 2.0).into();
        let current_position: Point3<f32> = self.position.current().into();
        ModelAttributes {
            position: current_position,
            rotation: self.rotation.current().into(),
            center,
            world_scale: self.world_scale,
        }
    }

    // 文字と caret の GPU で描画すべき位置やモーションを計算する
    #[inline]
    fn calc_instance_positions(&mut self, char_width_calcurator: &CharWidthCalculator) {
        let bound_in_animation = self.bound.in_animation();
        let position_in_animation = self.position.in_animation();
        let rotataion_in_animation = self.rotation.in_animation();
        let update_environment = self.config_updated
            || position_in_animation
            || bound_in_animation
            || rotataion_in_animation;

        let model_attributes = self.model_attributes();

        // update caret
        self.caret_states.update_instances(
            update_environment,
            &model_attributes,
            char_width_calcurator,
            &self.config,
        );

        // update chars
        self.char_states.update_instances(
            update_environment,
            &model_attributes,
            char_width_calcurator,
            &self.config,
        );
    }

    fn max_display_width(&self) -> usize {
        (self.config.max_col as f32 / self.config.col_interval).abs() as usize
    }

    pub(crate) fn set_config(&mut self, config: TextContext) {
        // direction が変わった場合は char_states の direction も更新する
        self.char_states.instances.set_direction(&config.direction);
        self.config = config;
        self.position.update_duration_and_easing_func(
            self.config.char_easings.position_easing.duration,
            self.config.char_easings.position_easing.easing_func,
        );
        self.bound.update_duration_and_easing_func(
            self.config.char_easings.position_easing.duration,
            self.config.char_easings.position_easing.easing_func,
        );
        self.config_updated = true;
    }

    pub(crate) fn direction(&self) -> Direction {
        self.config.direction
    }

    pub(crate) fn set_world_scale(&mut self, world_scale: [f32; 2]) {
        self.world_scale = world_scale;
    }
}

pub enum TextEditOperation {
    // テーマカラーを Range の範囲で設定する
    SetThemedColor(Range<CellPosition>, ThemedColor),
}
