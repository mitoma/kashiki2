use std::sync::mpsc::Receiver;

use cgmath::{Point2, Point3, Quaternion, Rotation3};

use instant::Duration;
use text_buffer::{
    caret::CaretType,
    editor::{ChangeEvent, Editor, LineBoundaryProhibitedChars, PhisicalLayout},
};

use crate::{
    char_width_calcurator::CharWidth,
    color_theme::ColorTheme,
    easing_value::EasingPointN,
    font_buffer::{Direction, GlyphVertexBuffer},
    instances::GlyphInstances,
    layout_engine::{Model, ModelOperation, ModelOperationResult},
};

use super::{
    caret_char,
    view_element_state::{CaretStates, CharStates},
};

pub struct EasingConfig {
    duration: Duration,
    easing_func: fn(f32) -> f32,
}

pub struct TextEditConfig {
    pub(crate) direction: Direction,
    pub(crate) row_interval: f32,
    pub(crate) col_interval: f32,
    pub(crate) max_col: usize,
    pub(crate) line_prohibited_chars: LineBoundaryProhibitedChars,
    pub(crate) min_bound: Point2<f32>,
    pub(crate) position_easing: EasingConfig,
    pub(crate) color_theme: ColorTheme,
    pub(crate) psychedelic: bool,
}

impl Default for TextEditConfig {
    fn default() -> Self {
        Self {
            direction: Direction::Horizontal,
            row_interval: 1.0,
            col_interval: 0.7,
            max_col: 40,
            line_prohibited_chars: LineBoundaryProhibitedChars::default(),
            min_bound: (10.0, 10.0).into(),
            position_easing: EasingConfig {
                duration: Duration::from_millis(800),
                easing_func: nenobi::functions::sin_in_out,
            },
            color_theme: ColorTheme::SolarizedDark,
            psychedelic: false,
        }
    }
}

pub struct TextEdit {
    config: TextEditConfig,

    editor: Editor,
    receiver: Receiver<ChangeEvent>,

    char_states: CharStates,
    caret_states: CaretStates,

    text_updated: bool,
    config_updated: bool,

    position: EasingPointN<3>,
    rotation: Quaternion<f32>,
    bound: EasingPointN<2>,
}

impl Default for TextEdit {
    fn default() -> Self {
        let config = TextEditConfig::default();
        let (tx, rx) = std::sync::mpsc::channel();

        let mut position = EasingPointN::new([0.0, 0.0, 0.0]);
        position.update_duration_and_easing_func(
            config.position_easing.duration,
            config.position_easing.easing_func,
        );
        let bound = config.min_bound.into();
        Self {
            config,
            editor: Editor::new(tx),
            receiver: rx,

            char_states: CharStates::default(),
            caret_states: CaretStates::default(),

            text_updated: true,
            config_updated: true,

            position,
            rotation: cgmath::Quaternion::from_axis_angle(
                cgmath::Vector3::unit_y(),
                cgmath::Deg(0.0),
            ),
            bound,
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
        if self.rotation == rotation {
            return;
        }
        self.rotation = rotation;
        self.config_updated = true;
    }

    fn rotation(&self) -> Quaternion<f32> {
        self.rotation
    }

    fn bound(&self) -> (f32, f32) {
        // 外向けにはアニメーション完了後の最終的なサイズを返す
        // この値はレイアウトの計算に使われるためである
        self.bound.last().into()
    }

    fn glyph_instances(&self) -> Vec<&GlyphInstances> {
        [
            self.caret_states.instances.to_instances(),
            self.char_states.instances.to_instances(),
        ]
        .concat()
    }

    fn update(
        &mut self,
        color_theme: &crate::color_theme::ColorTheme,
        glyph_vertex_buffer: &mut crate::font_buffer::GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        if self.config.color_theme != *color_theme {
            self.config.color_theme = *color_theme;
            self.char_states.update_char_theme(color_theme);
            self.config_updated = true;
        }

        self.sync_editor_events(device, color_theme);

        if self.text_updated {
            let layout = self.calc_phisical_layout(glyph_vertex_buffer);
            let bound = self.calc_bound(&layout);
            self.calc_position(glyph_vertex_buffer, &layout, bound);
        }

        self.calc_instance_positions(glyph_vertex_buffer);
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
            ModelOperation::ChangeDirection => {
                match self.config.direction {
                    Direction::Horizontal => self.config.direction = Direction::Vertical,
                    Direction::Vertical => self.config.direction = Direction::Horizontal,
                }
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
            ModelOperation::CopyDisplayString(width_resolver, result_callback) => {
                result_callback(
                    self.editor
                        .calc_phisical_layout(
                            self.max_display_width(),
                            &self.config.line_prohibited_chars,
                            *width_resolver,
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
        }
    }

    fn to_string(&self) -> String {
        self.editor.to_buffer_string()
    }
}

impl TextEdit {
    // editor から受け取ったイベントを TextEdit の caret, buffer_chars, instances に同期する。
    #[inline]
    fn sync_editor_events(&mut self, device: &wgpu::Device, color_theme: &ColorTheme) {
        while let Ok(event) = self.receiver.try_recv() {
            self.text_updated = true;
            match event {
                ChangeEvent::AddChar(c) => {
                    let caret_pos = self
                        .caret_states
                        .main_caret_position()
                        .map(|[x, y, z]| [x, y + 1.0, z])
                        .unwrap_or_else(|| [0.0, 1.0, 0.0]);
                    self.char_states
                        .add_char(c, caret_pos, color_theme.text().get_color(), device);
                }
                ChangeEvent::MoveChar { from, to } => {
                    self.char_states.move_char(from, to, device);
                }
                ChangeEvent::RemoveChar(c) => {
                    self.char_states.char_to_dustbox(c);
                }
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
    }

    #[inline]
    fn calc_phisical_layout(&mut self, glyph_vertex_buffer: &GlyphVertexBuffer) -> PhisicalLayout {
        self.editor.calc_phisical_layout(
            self.max_display_width(),
            &self.config.line_prohibited_chars,
            glyph_vertex_buffer,
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

    // 文字と caret の x, y の model 上の位置を計算
    #[inline]
    fn calc_position(
        &mut self,
        glyph_vertex_buffer: &GlyphVertexBuffer,
        layout: &PhisicalLayout,
        bound: [f32; 2],
    ) {
        // update char position
        layout.chars.iter().for_each(|(c, pos)| {
            let width = glyph_vertex_buffer.width(c.c);
            let position =
                Self::get_adjusted_position(&self.config, width, bound, [pos.col, pos.row]);
            self.char_states.update_state_position(c, position)
        });

        // update caret position
        {
            let caret_width = glyph_vertex_buffer.width(caret_char(CaretType::Primary));
            let position = Self::get_adjusted_position(
                &self.config,
                caret_width,
                bound,
                [layout.main_caret_pos.col, layout.main_caret_pos.row],
            );
            self.caret_states
                .update_state_position(CaretType::Primary, position);
        }
        if let Some(mark_pos) = layout.mark_pos {
            let caret_width = glyph_vertex_buffer.width(caret_char(CaretType::Mark));
            let position = Self::get_adjusted_position(
                &self.config,
                caret_width,
                bound,
                [mark_pos.col, mark_pos.row],
            );
            self.caret_states
                .update_state_position(CaretType::Mark, position);
        }

        // update selection color
        // FIXME: 毎回 selection を使って状態を更新するのではなく text editor からのイベントで更新するようにするのが望ましい
        if let Some((from, to)) = self.caret_states.get_selection() {
            self.char_states
                .apply_selection_color(from, to, &self.config);
        } else {
            self.char_states.clear_selection_color(&self.config);
        }
    }

    #[inline]
    fn get_adjusted_position(
        config: &TextEditConfig,
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

    // 文字と caret の GPU で描画すべき位置やモーションを計算する
    #[inline]
    fn calc_instance_positions(&mut self, glyph_vertex_buffer: &GlyphVertexBuffer) {
        let bound_in_animation = self.bound.in_animation();
        let [bound_x, bound_y] = &self.bound.current();
        let center = (bound_x / 2.0, -bound_y / 2.0).into();
        let position_in_animation = self.position.in_animation();
        let current_position: Point3<f32> = self.position.current().into();
        let update_environment = position_in_animation || bound_in_animation || self.config_updated;

        // update caret
        self.caret_states.update_instances(
            update_environment,
            &center,
            &current_position,
            &self.rotation,
            glyph_vertex_buffer,
            &self.config,
        );

        // update chars
        self.char_states.update_instances(
            update_environment,
            &center,
            &current_position,
            &self.rotation,
            glyph_vertex_buffer,
            &self.config,
        );
    }

    fn max_display_width(&self) -> usize {
        (self.config.max_col as f32 / self.config.col_interval).abs() as usize
    }
}
