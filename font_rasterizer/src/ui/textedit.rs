use std::{collections::BTreeMap, sync::mpsc::Receiver};

use cgmath::{Point2, Point3, Quaternion, Rotation3};

use instant::Duration;
use log::info;
use text_buffer::{
    buffer::BufferChar,
    caret::{Caret, CaretType},
    editor::{ChangeEvent, Editor, LineBoundaryProhibitedChars},
};

use crate::{
    color_theme,
    easing_value::{EasingPoint2, EasingPoint3},
    font_buffer::{Direction, GlyphVertexBuffer},
    font_converter::GlyphWidth,
    instances::{GlyphInstance, GlyphInstances},
    layout_engine::{Model, ModelOperation},
    motion::MotionFlags,
    text_instances::TextInstances,
};

pub struct TextEdit {
    editor: Editor,
    direction: Direction,
    receiver: Receiver<ChangeEvent>,
    buffer_chars: BTreeMap<BufferChar, EasingPoint3>,
    removed_buffer_chars: BTreeMap<BufferChar, EasingPoint3>,
    main_caret: Option<(Caret, EasingPoint3)>,
    mark: Option<(Caret, EasingPoint3)>,
    removed_carets: BTreeMap<Caret, EasingPoint3>,
    motion: MotionFlags,
    instances: TextInstances,
    caret_instances: TextInstances,
    text_updated: bool,
    position: EasingPoint3,
    rotation: Quaternion<f32>,
    bound: EasingPoint2,
    char_interval: f32,
}

impl Default for TextEdit {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

        let mut position = EasingPoint3::new(0.0, 0.0, 0.0);
        position.update_duration_and_easing_func(
            Duration::from_millis(800),
            nenobi::functions::sin_in_out,
        );
        Self {
            editor: Editor::new(tx),
            direction: Direction::Horizontal,
            receiver: rx,
            buffer_chars: BTreeMap::new(),
            removed_buffer_chars: BTreeMap::new(),
            main_caret: None,
            mark: None,
            removed_carets: BTreeMap::new(),
            motion: MotionFlags::ZERO_MOTION,
            instances: TextInstances::default(),
            caret_instances: TextInstances::default(),
            text_updated: true,
            position,
            rotation: cgmath::Quaternion::from_axis_angle(
                cgmath::Vector3::unit_y(),
                cgmath::Deg(0.0),
            ),
            bound: EasingPoint2::new(20.0, 20.0),
            char_interval: 0.8,
        }
    }
}

impl Model for TextEdit {
    fn set_position(&mut self, position: Point3<f32>) {
        if self.position.last() == position.into() {
            return;
        }
        self.position.update(position);
    }

    fn position(&self) -> Point3<f32> {
        let caret_position = self
            .main_caret
            .as_ref()
            .map(|(_, c)| {
                let (x, y, z) = c.last();
                Point3::new(x, y, z)
            })
            .unwrap_or_else(|| Point3::new(0.0, 0.0, 0.0));

        let position: Point3<f32> = self.position.last().into();
        let current_bound = self.bound.last();
        match self.direction {
            Direction::Horizontal => Point3::new(
                position.x,
                position.y + caret_position.y + current_bound.1 / 2.0,
                position.z,
            ),
            Direction::Vertical => Point3::new(
                position.x + caret_position.x - current_bound.0 / 2.0,
                position.y,
                position.z,
            ),
        }
    }

    fn set_rotation(&mut self, rotation: Quaternion<f32>) {
        if self.rotation == rotation {
            return;
        }
        self.rotation = rotation;
        // FIXME 意味が違うので後でなおす
        self.text_updated = true;
    }

    fn rotation(&self) -> Quaternion<f32> {
        self.rotation
    }

    fn bound(&self) -> (f32, f32) {
        // 外向けには最終のサイズを出す
        self.bound.last()
    }

    fn glyph_instances(&self) -> Vec<&GlyphInstances> {
        let mut char_insatnces = self.instances.to_instances();
        let mut caret_instances = self.caret_instances.to_instances();
        char_insatnces.append(&mut caret_instances);
        char_insatnces
    }

    fn update(
        &mut self,
        color_theme: &crate::color_theme::ColorTheme,
        glyph_vertex_buffer: &mut crate::font_buffer::GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.sync_editor_events(device, color_theme);
        self.calc_position_and_bound(glyph_vertex_buffer);
        self.calc_instance_positions(glyph_vertex_buffer);
        self.instances.update(device, queue);
        self.text_updated = false;
    }

    fn editor_operation(&mut self, op: &text_buffer::action::EditorOperation) {
        self.editor.operation(op)
    }

    fn model_operation(&mut self, op: &ModelOperation) {
        match op {
            ModelOperation::ChangeDirection => {
                match self.direction {
                    Direction::Horizontal => self.direction = Direction::Vertical,
                    Direction::Vertical => self.direction = Direction::Horizontal,
                }
                self.instances.set_direction(&self.direction);
                self.text_updated = true;
            }
            ModelOperation::IncreaseCharInterval => {
                self.char_interval += 0.05;
                self.text_updated = true;
            }
            ModelOperation::DecreaseCharInterval => {
                self.char_interval -= 0.05;
                self.text_updated = true;
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
    fn sync_editor_events(&mut self, device: &wgpu::Device, color_theme: &color_theme::ColorTheme) {
        while let Ok(event) = self.receiver.try_recv() {
            self.text_updated = true;
            match event {
                ChangeEvent::AddChar(c) => {
                    let caret_pos = self
                        .main_caret
                        .as_ref()
                        .map(|(_, c)| {
                            let (x, y, z) = c.current();
                            (x, y + 1.0, z)
                        })
                        .unwrap_or_else(|| (0.0, 1.0, 0.0));
                    self.buffer_chars.insert(c, caret_pos.into());
                    let instance = GlyphInstance {
                        color: color_theme.text().get_color(),
                        motion: self.motion,
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
                        ..GlyphInstance::default()
                    };
                    self.caret_instances.add(c.into(), caret_instance, device);
                    if c.caret_type == CaretType::Primary {
                        self.main_caret = Some((c, (c.row as f32, c.col as f32, 0.0).into()));
                    } else {
                        self.mark = Some((c, (c.row as f32, c.col as f32, 0.0).into()));
                    }
                }
                ChangeEvent::MoveCaret { from, to } => {
                    if let Some(instance) = self.caret_instances.remove(&from.into()) {
                        self.caret_instances.add(to.into(), instance, device);
                    }
                }
                ChangeEvent::RemoveCaret(c) => {
                    match CaretType::Primary {
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

    // 文字と caret の x, y の論理的な位置を計算する
    #[inline]
    fn calc_position_and_bound(&mut self, glyph_vertex_buffer: &GlyphVertexBuffer) {
        if !self.text_updated {
            return;
        }
        let max_line_width = 40;

        let layout = self.editor.calc_phisical_layout(
            (max_line_width as f32 / self.char_interval).abs() as usize,
            &LineBoundaryProhibitedChars::new(vec![], vec![]),
            glyph_vertex_buffer,
        );

        {
            // update bound
            let (max_col, max_row) = layout.chars.iter().fold((0, 0), |result, (_, pos)| {
                (result.0.max(pos.col), result.1.max(pos.row))
            });
            let (max_x, max_y, _) = Self::get_adjusted_position(
                self.direction,
                self.char_interval,
                0.0,
                (max_col as f32, max_row as f32, 0.0),
            );
            self.bound.update((max_x.abs(), max_y.abs()).into());
        }

        layout.chars.iter().for_each(|(c, pos)| {
            if let Some(c_pos) = self.buffer_chars.get_mut(c) {
                let width = glyph_vertex_buffer.width(c.c);
                c_pos.update(
                    Self::get_adjusted_position(
                        self.direction,
                        self.char_interval,
                        max_line_width as f32,
                        ((pos.col as f32 / 2.0) + width.left(), pos.row as f32, 0.0),
                    )
                    .into(),
                );
            }
        });

        let caret_width = glyph_vertex_buffer.width('_');
        if let Some((_, c)) = self.main_caret.as_mut() {
            info!("main_caret_pos layout: {:?}", layout.main_caret_pos);
            info!("main_caret_pos easing: {:?}", c.last());
            c.update(
                Self::get_adjusted_position(
                    self.direction,
                    self.char_interval,
                    max_line_width as f32,
                    (
                        (layout.main_caret_pos.col as f32 / 2.0) + caret_width.left(),
                        layout.main_caret_pos.row as f32,
                        0.0,
                    ),
                )
                .into(),
            );
        }

        if let (Some((_, c)), Some(mark_pos)) = (self.mark.as_mut(), layout.mark_pos) {
            c.update(
                Self::get_adjusted_position(
                    self.direction,
                    self.char_interval,
                    max_line_width as f32,
                    (
                        (mark_pos.col as f32 / 2.0) + caret_width.left(),
                        mark_pos.row as f32,
                        0.0,
                    ),
                )
                .into(),
            )
        }
    }

    #[inline]
    fn get_adjusted_position(
        direction: Direction,
        char_interval: f32,
        max_width: f32,
        (x, y, z): (f32, f32, f32),
    ) -> (f32, f32, f32) {
        let x = x * char_interval;
        match direction {
            Direction::Horizontal => (x, -y, z),
            Direction::Vertical => (max_width - y, -x, z),
        }
    }

    // 文字と caret の GPU で描画すべき位置やモーションを計算する
    #[inline]
    fn calc_instance_positions(&mut self, glyph_vertex_buffer: &GlyphVertexBuffer) {
        let bound_in_animation = self.bound.in_animation();
        let (bound_x, bound_y) = &self.bound.current();
        let center = (bound_x / 2.0, -bound_y / 2.0).into();
        let position_in_animation = self.position.in_animation();
        let current_position: Point3<f32> = self.position.current().into();

        // update caret
        info!("caret_instances len: {}", self.caret_instances.len());
        if let Some((c, i)) = self.main_caret.as_mut() {
            if Self::dismiss_update(i, position_in_animation, bound_in_animation) {
                //
            } else if let Some(instance) = self.caret_instances.get_mut(&c.clone().into()) {
                info!("update position: {:?}", i.current());
                info!("update position: {:?}", i.last());
                Self::update_instance(
                    instance,
                    i,
                    &center,
                    &current_position,
                    &self.rotation,
                    None,
                );
            }
        }
        if let Some((c, i)) = self.mark.as_mut() {
            if Self::dismiss_update(i, position_in_animation, bound_in_animation) {
                //
            } else if let Some(instance) = self.caret_instances.get_mut(&c.clone().into()) {
                Self::update_instance(
                    instance,
                    i,
                    &center,
                    &current_position,
                    &self.rotation,
                    None,
                );
            }
        }

        // update chars
        for (c, i) in self.buffer_chars.iter_mut() {
            if Self::dismiss_update(i, position_in_animation, bound_in_animation) {
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
                    Self::calc_rotation(c.c, self.direction, glyph_vertex_buffer),
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
                    Self::calc_rotation(c.c, self.direction, glyph_vertex_buffer),
                );
            }
        }
    }

    #[inline]
    fn calc_rotation(
        c: char,
        direction: Direction,
        glyph_vertex_buffer: &GlyphVertexBuffer,
    ) -> Option<Quaternion<f32>> {
        match direction {
            Direction::Horizontal => None,
            Direction::Vertical => {
                let width = glyph_vertex_buffer.width(c);
                match width {
                    GlyphWidth::Regular => Some(cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_z(),
                        cgmath::Deg(-90.0),
                    )),
                    GlyphWidth::Wide => None,
                }
            }
        }
    }

    // この関数は更新が必要かどうかを判定するための関数。true なら更新が不要。
    #[inline]
    fn dismiss_update(
        easiong_point: &mut EasingPoint3,
        position_in_animation: bool,
        bound_in_animation: bool,
    ) -> bool {
        !easiong_point.in_animation() && !position_in_animation && !bound_in_animation
    }

    #[inline]
    fn update_instance(
        instance: &mut GlyphInstance,
        i: &EasingPoint3,
        center: &Point2<f32>,
        position: &Point3<f32>,
        rotation: &Quaternion<f32>,
        char_rotation: Option<Quaternion<f32>>,
    ) {
        let (x, y, z) = i.current();
        let pos = cgmath::Matrix4::from(*rotation)
            * cgmath::Matrix4::from_translation(cgmath::Vector3 { x, y, z }).w;
        let new_position = cgmath::Vector3 {
            // FIXME center の値をさらに半分にしているのは前の段階で
            // 半角のサイズを 1 → 1.0 に変換しているのが原因と思われる。いい感じに直してくれ。
            x: pos.x - (center.x / 2.0) + position.x,
            y: pos.y - (center.y / 2.0) + position.y,
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
