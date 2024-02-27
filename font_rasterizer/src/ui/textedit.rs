use std::{collections::BTreeMap, sync::mpsc::Receiver};

use cgmath::{Point2, Point3, Quaternion, Rotation3};

use instant::Duration;
use log::debug;
use text_buffer::{
    buffer::BufferChar,
    caret::Caret,
    editor::{ChangeEvent, Editor},
};

use crate::{
    color_theme,
    easing_value::EasingPoint3,
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
    carets: BTreeMap<Caret, EasingPoint3>,
    removed_carets: BTreeMap<Caret, EasingPoint3>,
    motion: MotionFlags,
    instances: TextInstances,
    text_updated: bool,
    position: EasingPoint3,
    rotation: Quaternion<f32>,
    bound: (f32, f32),
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
            carets: BTreeMap::new(),
            removed_carets: BTreeMap::new(),
            motion: MotionFlags::ZERO_MOTION,
            instances: TextInstances::default(),
            text_updated: true,
            position,
            rotation: cgmath::Quaternion::from_axis_angle(
                cgmath::Vector3::unit_y(),
                cgmath::Deg(0.0),
            ),
            bound: (20.0, 20.0),
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
            .carets
            .iter()
            // TODO Mark ではない caret を判定する方法が unique_key の大小で行うのはあまりよろしい実装ではないのでいずれ見直す
            .min_by(|(l, _), (r, _)| l.unique_key.cmp(&r.unique_key))
            .map(|(_, c)| {
                let (x, y, z) = c.last();
                Point3::new(x, y, z)
            })
            .unwrap_or_else(|| Point3::new(0.0, 0.0, 0.0));

        let position: Point3<f32> = self.position.last().into();
        match self.direction {
            Direction::Horizontal => Point3::new(
                position.x,
                position.y + caret_position.y + self.bound.1 / 2.0,
                position.z,
            ),
            Direction::Vertical => Point3::new(
                position.x + caret_position.x - self.bound.0 / 2.0,
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
        self.bound
    }

    fn glyph_instances(&self) -> Vec<&GlyphInstances> {
        self.instances.to_instances()
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
                        .carets
                        .first_key_value()
                        .map(|(_, c)| {
                            let (x, y, z) = c.current();
                            (x, y + 1.0, z)
                        })
                        .unwrap_or_else(|| (0.0, 0.0, 0.0));
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
                    self.carets
                        .insert(c, (c.row as f32, c.col as f32, 0.0).into());
                    self.instances.add(c.into(), caret_instance, device);
                }
                ChangeEvent::MoveCaret { from, to } => {
                    debug!("MoveCaret: from: {:?}, to: {:?}", from, to);
                    if let Some(position) = self.carets.remove(&from) {
                        self.carets.insert(to, position);
                    }
                    if let Some(instance) = self.instances.remove(&from.into()) {
                        self.instances.add(to.into(), instance, device);
                    }
                }
                ChangeEvent::RemoveCaret(c) => {
                    if let Some(position) = self.carets.remove(&c) {
                        self.removed_carets.insert(c, position);
                    }
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
        let caret_width = glyph_vertex_buffer.width('_');

        let max_width = self.bound.0;
        let initial_x: f32 = 0.0;
        let mut current_row: usize = 0;
        let mut x: f32 = 0.0;
        let mut y: f32 = 0.0;

        if let Some((_, caret_position)) = self
            .carets
            .iter_mut()
            .find(|(caret, _)| caret.row == 0 && caret.col == 0)
        {
            let caret_x = initial_x + caret_width.left();
            let caret_y = y;
            caret_position.update(
                Self::get_adjusted_position(self.direction, max_width, (caret_x, caret_y, 0.0))
                    .into(),
            );
        }

        // caret の位置決め
        // 文字と caret (文中あるいは文末時の位置決め)
        for (c, i) in self.buffer_chars.iter_mut() {
            // 行が変わっている時は、行の先頭に caret を移動させる
            if current_row != c.row {
                for r in ((current_row + 1)..=(c.row)).rev() {
                    if let Some((_, caret_position)) = self
                        .carets
                        .iter_mut()
                        .find(|(caret, _)| caret.row == r && caret.col == 0)
                    {
                        let caret_x = initial_x + caret_width.left();
                        let caret_y = y - (r - current_row) as f32;
                        caret_position.update(
                            Self::get_adjusted_position(
                                self.direction,
                                max_width,
                                (caret_x, caret_y, 0.0),
                            )
                            .into(),
                        );
                    }
                }

                let y_gain = c.row - current_row;
                current_row = c.row;
                y -= 1.0 * y_gain as f32;
                x = initial_x;
            }
            if x >= max_width {
                y -= 1.0;
                x = initial_x;
            }

            let glyph_width = glyph_vertex_buffer.width(c.c);
            x += glyph_width.left() * self.char_interval;
            i.update(Self::get_adjusted_position(self.direction, max_width, (x, y, 0.0)).into());

            if let Some((_, caret_position)) = self
                .carets
                .iter_mut()
                .find(|(caret, _)| caret.row == c.row && caret.col == c.col)
            {
                caret_position.update(
                    Self::get_adjusted_position(self.direction, max_width, (x, y, 0.0)).into(),
                );
            }
            x += glyph_width.right() * self.char_interval;
            if let Some((_, caret_position)) = self
                .carets
                .iter_mut()
                .find(|(caret, _)| caret.row == c.row && caret.col == c.col + 1)
            {
                caret_position.update(
                    Self::get_adjusted_position(
                        self.direction,
                        max_width,
                        (x + caret_width.left(), y, 0.0),
                    )
                    .into(),
                );
            }
        }

        for (c, i) in self.carets.iter_mut() {
            if current_row < c.row {
                let caret_x = initial_x + caret_width.left();
                let caret_y = y - (c.row - current_row) as f32;
                i.update(
                    Self::get_adjusted_position(self.direction, max_width, (caret_x, caret_y, 0.0))
                        .into(),
                );
            }
        }
    }

    fn get_adjusted_position(
        direction: Direction,
        max_width: f32,
        position: (f32, f32, f32),
    ) -> (f32, f32, f32) {
        match direction {
            Direction::Horizontal => position,
            Direction::Vertical => (max_width + position.1, -position.0, position.2),
        }
    }

    // 文字と caret の GPU で描画すべき位置やモーションを計算する
    #[inline]
    fn calc_instance_positions(&mut self, glyph_vertex_buffer: &GlyphVertexBuffer) {
        let (bound_x, bound_y) = &self.bound;
        let center = (bound_x / 2.0, -bound_y / 2.0).into();
        let position_in_animation = self.position.in_animation();
        let current_position: Point3<f32> = self.position.current().into();

        // update caret
        for (c, i) in self.carets.iter_mut() {
            if Self::dismiss_update(i, position_in_animation) {
                continue;
            }
            if let Some(instance) = self.instances.get_mut(&(*c).into()) {
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
            if Self::dismiss_update(i, position_in_animation) {
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

    #[inline]
    fn dismiss_update(easiong_point: &mut EasingPoint3, position_in_animation: bool) -> bool {
        !easiong_point.in_animation() && !position_in_animation
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
            x: pos.x - center.x + position.x,
            y: pos.y - center.y + position.y,
            z: pos.z + position.z,
        };
        instance.position = new_position;
        instance.rotation = match char_rotation {
            Some(r) => *rotation * r,
            None => *rotation,
        }
    }
}
