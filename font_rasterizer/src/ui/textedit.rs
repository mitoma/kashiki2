use std::{collections::BTreeMap, sync::mpsc::Receiver};

use cgmath::{Point2, Point3, Quaternion, Rotation3};

use instant::Duration;
use text_buffer::{
    buffer::BufferChar,
    caret::{Caret, CaretType},
    editor::{ChangeEvent, Editor, LineBoundaryProhibitedChars},
};

use crate::{
    char_width_calcurator::CharWidth,
    color_theme::ColorTheme,
    easing_value::EasingPointN,
    font_buffer::{Direction, GlyphVertexBuffer},
    instances::{GlyphInstance, GlyphInstances},
    layout_engine::{Model, ModelOperation, ModelOperationResult},
    motion::MotionFlags,
    text_instances::TextInstances,
};

use super::caret_char;

pub struct EasingConfig {
    duration: Duration,
    easing_func: fn(f32) -> f32,
}

pub struct TextEditConfig {
    direction: Direction,
    row_interval: f32,
    col_interval: f32,
    max_col: usize,
    line_prohibited_chars: LineBoundaryProhibitedChars,
    min_bound: Point2<f32>,
    position_easing: EasingConfig,
    char_motion: MotionFlags,
    caret_motion: MotionFlags,
    color_theme: ColorTheme,
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
            char_motion: MotionFlags::ZERO_MOTION,
            caret_motion: MotionFlags::ZERO_MOTION,
            color_theme: ColorTheme::SolarizedDark,
        }
    }
}

pub struct TextEdit {
    config: TextEditConfig,

    editor: Editor,
    receiver: Receiver<ChangeEvent>,

    buffer_chars: BTreeMap<BufferChar, EasingPointN<3>>,
    removed_buffer_chars: BTreeMap<BufferChar, EasingPointN<3>>,
    instances: TextInstances,

    main_caret: Option<(Caret, EasingPointN<3>)>,
    mark: Option<(Caret, EasingPointN<3>)>,
    removed_carets: BTreeMap<Caret, EasingPointN<3>>,
    caret_instances: TextInstances,

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

            buffer_chars: BTreeMap::new(),
            removed_buffer_chars: BTreeMap::new(),
            instances: TextInstances::default(),

            main_caret: None,
            mark: None,
            removed_carets: BTreeMap::new(),
            caret_instances: TextInstances::default(),

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
            .main_caret
            .as_ref()
            .map(|(_, c)| c.last())
            .unwrap_or_else(|| [0.0, 0.0, 0.0]);

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
            self.caret_instances.to_instances(),
            self.instances.to_instances(),
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
            self.config_updated = true;
        }

        self.sync_editor_events(device, color_theme);
        self.calc_position_and_bound(glyph_vertex_buffer);
        self.calc_instance_positions(glyph_vertex_buffer);
        self.instances.update(device, queue);
        self.caret_instances.update(device, queue);
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
                self.instances.set_direction(&self.config.direction);
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
                        .main_caret
                        .as_ref()
                        .map(|(_, c)| {
                            let [x, y, z] = c.current();
                            [x, y + 1.0, z]
                        })
                        .unwrap_or_else(|| [0.0, 1.0, 0.0]);
                    self.buffer_chars.insert(c, caret_pos.into());
                    let instance = GlyphInstance {
                        color: color_theme.text().get_color(),
                        motion: self.config.char_motion,
                        ..GlyphInstance::default()
                    };
                    self.instances.add(c.into(), instance, device);
                }
                ChangeEvent::MoveChar { from, to } => {
                    if let Some(position) = self.buffer_chars.remove(&from) {
                        self.buffer_chars.insert(to, position);
                    }
                    if let Some(instance) = self.instances.remove(&from.into()) {
                        self.instances.add(to.into(), instance, device);
                    }
                }
                ChangeEvent::RemoveChar(c) => {
                    if let Some(mut position) = self.buffer_chars.remove(&c) {
                        position.add((0.0, -1.0, 0.0).into());
                        self.removed_buffer_chars.insert(c, position);
                    }
                    self.instances.pre_remove(&c.into());
                }
                ChangeEvent::AddCaret(c) => {
                    let caret_instance = GlyphInstance {
                        color: color_theme.text_emphasized().get_color(),
                        motion: self.config.caret_motion,
                        ..GlyphInstance::default()
                    };
                    self.caret_instances.add(c.into(), caret_instance, device);
                    if c.caret_type == CaretType::Primary {
                        self.main_caret = Some((c, [c.row as f32, c.col as f32, 0.0].into()));
                    } else {
                        self.mark = Some((c, [c.row as f32, c.col as f32, 0.0].into()));
                    }
                }
                ChangeEvent::MoveCaret { from, to } => {
                    match from.caret_type {
                        CaretType::Primary => {
                            self.main_caret = Some((to, self.main_caret.take().unwrap().1))
                        }
                        CaretType::Mark => self.mark = Some((to, self.mark.take().unwrap().1)),
                    }
                    if let Some(instance) = self.caret_instances.remove(&from.into()) {
                        self.caret_instances.add(to.into(), instance, device);
                    }
                }
                ChangeEvent::RemoveCaret(c) => {
                    match c.caret_type {
                        CaretType::Primary => {
                            if let Some((_, mut position)) = self.main_caret.take() {
                                position.add((0.0, -1.0, 0.0).into());
                                self.removed_carets.insert(c, position);
                            }
                        }
                        CaretType::Mark => {
                            if let Some((_, mut position)) = self.mark.take() {
                                position.add((0.0, -1.0, 0.0).into());
                                self.removed_carets.insert(c, position);
                            }
                        }
                    }
                    self.caret_instances.pre_remove(&c.into());
                }
            }
        }
    }

    // 文字と caret の x, y の論理的な位置を計算し、bound を更新する
    #[inline]
    fn calc_position_and_bound(&mut self, glyph_vertex_buffer: &GlyphVertexBuffer) {
        if !self.text_updated {
            return;
        }

        let layout = self.editor.calc_phisical_layout(
            (self.config.max_col as f32 / self.config.col_interval).abs() as usize,
            &self.config.line_prohibited_chars,
            glyph_vertex_buffer,
        );

        let bound = {
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
        };

        layout.chars.iter().for_each(|(c, pos)| {
            if let Some(c_pos) = self.buffer_chars.get_mut(c) {
                let width = glyph_vertex_buffer.width(c.c);
                c_pos.update(Self::get_adjusted_position(
                    &self.config,
                    width,
                    bound,
                    [pos.col, pos.row],
                ));
            }
        });

        if let Some((caret, c)) = self.main_caret.as_mut() {
            let caret_width = glyph_vertex_buffer.width(caret_char(caret));
            c.update(Self::get_adjusted_position(
                &self.config,
                caret_width,
                bound,
                [layout.main_caret_pos.col, layout.main_caret_pos.row],
            ));
        }

        if let (Some((caret, c)), Some(mark_pos)) = (self.mark.as_mut(), layout.mark_pos) {
            let caret_width = glyph_vertex_buffer.width(caret_char(caret));
            c.update(Self::get_adjusted_position(
                &self.config,
                caret_width,
                bound,
                [mark_pos.col, mark_pos.row],
            ))
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

        // update caret
        if let Some((c, i)) = self.main_caret.as_mut() {
            if Self::dismiss_update(
                i,
                position_in_animation,
                bound_in_animation,
                self.config_updated,
            ) {
                //
            } else if let Some(instance) = self.caret_instances.get_mut(&(*c).into()) {
                Self::update_instance(
                    instance,
                    i,
                    &center,
                    &current_position,
                    &self.rotation,
                    self.config.color_theme.text_emphasized().get_color(),
                    Self::calc_rotation(caret_char(c), &self.config, glyph_vertex_buffer),
                );
            }
        }
        if let Some((c, i)) = self.mark.as_mut() {
            if Self::dismiss_update(
                i,
                position_in_animation,
                bound_in_animation,
                self.config_updated,
            ) {
                //
            } else if let Some(instance) = self.caret_instances.get_mut(&(*c).into()) {
                Self::update_instance(
                    instance,
                    i,
                    &center,
                    &current_position,
                    &self.rotation,
                    self.config.color_theme.text_emphasized().get_color(),
                    Self::calc_rotation(caret_char(c), &self.config, glyph_vertex_buffer),
                );
            }
        }

        // update removed carets
        self.removed_carets.retain(|c, i| {
            let in_animation = i.in_animation();
            // こいつは消えゆく運命の Caret なので position_updated なんて考慮せずに in_animation だけ見る
            if !in_animation {
                self.caret_instances.remove_from_dustbox(&(*c).into());
            }
            in_animation
        });
        for (c, i) in self.removed_carets.iter() {
            if let Some(instance) = self.caret_instances.get_mut_from_dustbox(&(*c).into()) {
                Self::update_instance(
                    instance,
                    i,
                    &center,
                    &current_position,
                    &self.rotation,
                    self.config.color_theme.text_comment().get_color(),
                    Self::calc_rotation(caret_char(c), &self.config, glyph_vertex_buffer),
                );
            }
        }

        // update chars
        for (c, i) in self.buffer_chars.iter_mut() {
            if Self::dismiss_update(
                i,
                position_in_animation,
                bound_in_animation,
                self.config_updated,
            ) {
                continue;
            }
            if let Some(instance) = self.instances.get_mut(&(*c).into()) {
                // width が Reguler の時は rotation を 90 度回転させる
                Self::update_instance(
                    instance,
                    i,
                    &center,
                    &current_position,
                    &self.rotation,
                    self.config.color_theme.text().get_color(),
                    Self::calc_rotation(c.c, &self.config, glyph_vertex_buffer),
                );
            }
        }

        // update removed chars
        self.removed_buffer_chars.retain(|c, i| {
            let in_animation = i.in_animation();
            // こいつは消えゆく運命の文字なので position_updated なんて考慮せずに in_animation だけ見る
            if !in_animation {
                self.instances.remove_from_dustbox(&(*c).into());
            }
            in_animation
        });
        for (c, i) in self.removed_buffer_chars.iter() {
            if let Some(instance) = self.instances.get_mut_from_dustbox(&(*c).into()) {
                Self::update_instance(
                    instance,
                    i,
                    &center,
                    &current_position,
                    &self.rotation,
                    self.config.color_theme.text_comment().get_color(),
                    Self::calc_rotation(c.c, &self.config, glyph_vertex_buffer),
                );
            }
        }
    }

    #[inline]
    fn calc_rotation(
        c: char,
        config: &TextEditConfig,
        glyph_vertex_buffer: &GlyphVertexBuffer,
    ) -> Option<Quaternion<f32>> {
        match config.direction {
            Direction::Horizontal => None,
            Direction::Vertical => {
                let width = glyph_vertex_buffer.width(c);
                match width {
                    CharWidth::Regular => Some(cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_z(),
                        cgmath::Deg(-90.0),
                    )),
                    CharWidth::Wide => None,
                }
            }
        }
    }

    // この関数は更新が必要かどうかを判定するための関数。true なら更新が不要。
    #[inline]
    fn dismiss_update(
        easiong_point: &mut EasingPointN<3>,
        position_in_animation: bool,
        bound_in_animation: bool,
        config_updated: bool,
    ) -> bool {
        !easiong_point.in_animation()
            && !position_in_animation
            && !bound_in_animation
            && !config_updated
    }

    #[inline]
    fn update_instance(
        instance: &mut GlyphInstance,
        i: &EasingPointN<3>,
        center: &Point2<f32>,
        position: &Point3<f32>,
        rotation: &Quaternion<f32>,
        color: [f32; 3],
        char_rotation: Option<Quaternion<f32>>,
    ) {
        instance.color = color;
        let [x, y, z] = i.current();
        let pos = cgmath::Matrix4::from(*rotation)
            * cgmath::Matrix4::from_translation(cgmath::Vector3 { x, y, z }).w;
        let new_position = cgmath::Vector3 {
            x: pos.x - center.x + position.x,
            y: pos.y - center.y + position.y,
            z: pos.z + position.z,
        };
        instance.position = new_position;

        // 縦書きの場合は char_rotation が必要なのでここで回転する
        instance.rotation = match char_rotation {
            Some(r) => *rotation * r,
            None => *rotation,
        }
    }
}
