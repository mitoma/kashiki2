use std::{collections::BTreeMap, sync::mpsc::Receiver};

use cgmath::{Point2, Point3, Quaternion, Rotation3};

use text_buffer::{
    buffer::BufferChar,
    caret::Caret,
    editor::{ChangeEvent, Editor},
};

use crate::{
    color_theme,
    easing_value::EasingPoint3,
    font_buffer::{Direction, GlyphVertexBuffer},
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
    updated: bool,
    position: Point3<f32>,
    rotation: Quaternion<f32>,
    bound: (f32, f32),
}

impl Default for TextEdit {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

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
            updated: true,
            position: Point3::new(0.0, 0.0, 0.0),
            rotation: cgmath::Quaternion::from_axis_angle(
                cgmath::Vector3::unit_y(),
                cgmath::Deg(0.0),
            ),
            bound: (20.0, 20.0),
        }
    }
}

impl Model for TextEdit {
    fn set_position(&mut self, position: Point3<f32>) {
        if self.position == position {
            return;
        }
        self.position = position;
        self.updated = true;
    }

    fn position(&self) -> Point3<f32> {
        self.position
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
        self.calc_instance_positions();
        self.instances.update(device, queue);
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
                self.updated = true;
            }
        }
    }
}

impl TextEdit {
    // editor から受け取ったイベントを TextEdit の caret, buffer_chars, instances に同期する。
    #[inline]
    fn sync_editor_events(&mut self, device: &wgpu::Device, color_theme: &color_theme::ColorTheme) {
        while let Ok(event) = self.receiver.try_recv() {
            self.updated = true;
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
                    if let Some(position) = self.carets.remove(&from) {
                        self.carets.insert(to, position);
                    }
                    if let Some(instance) = self.instances.remove(&from.into()) {
                        self.instances.add(to.into(), instance, device);
                    }
                }
                ChangeEvent::RmoveCaret(c) => {
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
        if !self.updated {
            return;
        }
        let caret_width = glyph_vertex_buffer.width('_');

        let max_width = self.bound.0;
        let initial_x: f32 = 0.0;
        let mut current_row: usize = 0;
        let mut x: f32 = 0.0;
        let mut y: f32 = 0.0;

        if let Some(caret_position) = self.carets.get_mut(&Caret::new_without_event(0, 0)) {
            let caret_x = initial_x + caret_width.left();
            let caret_y = y;
            caret_position.update((caret_x, caret_y, 0.0).into());
        }

        // caret の位置決め
        // 文字と caret (文中あるいは文末時の位置決め)
        for (c, i) in self.buffer_chars.iter_mut() {
            // 行が変わっている時は、行の先頭に caret を移動させる
            if current_row != c.row {
                for r in ((current_row + 1)..=(c.row)).rev() {
                    if let Some(caret_position) =
                        self.carets.get_mut(&Caret::new_without_event(r, 0))
                    {
                        let caret_x = initial_x + caret_width.left();
                        let caret_y = y - (r - current_row) as f32;
                        caret_position.update((caret_x, caret_y, 0.0).into());
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
            x += glyph_width.left();
            i.update((x, y, 0.0).into());

            if let Some(caret_position) =
                self.carets.get_mut(&Caret::new_without_event(c.row, c.col))
            {
                caret_position.update((x, y, 0.0).into());
            }
            x += glyph_width.right();
            if let Some(caret_position) = self
                .carets
                .get_mut(&Caret::new_without_event(c.row, c.col + 1))
            {
                caret_position.update((x + caret_width.left(), y, 0.0).into());
            }
        }

        for (c, i) in self.carets.iter_mut() {
            if current_row < c.row {
                let caret_x = initial_x + caret_width.left();
                let caret_y = y - (c.row - current_row) as f32;
                i.update((caret_x, caret_y, 0.0).into());
            }
        }

        self.updated = false;
    }

    // 文字と caret の GPU で描画すべき位置やモーションを計算する
    #[inline]
    fn calc_instance_positions(&mut self) {
        let (bound_x, bound_y) = &self.bound;
        let center = (bound_x / 2.0, -bound_y / 2.0).into();

        // update caret
        for (c, i) in self.carets.iter_mut() {
            if !i.in_animation() {
                continue;
            }
            if let Some(instance) = self.instances.get_mut(&(*c).into()) {
                Self::update_instance(instance, i, &center, &self.position, &self.rotation);
            }
        }

        // update chars
        for (c, i) in self.buffer_chars.iter_mut() {
            if !i.in_animation() {
                continue;
            }
            if let Some(instance) = self.instances.get_mut(&(*c).into()) {
                Self::update_instance(instance, i, &center, &self.position, &self.rotation);
            }
        }

        // update removed chars
        self.removed_buffer_chars.retain(|c, i| {
            let in_animation = i.in_animation();
            if !in_animation {
                self.instances.remove_from_dustbox(&(*c).into());
            }
            in_animation
        });
        for (c, i) in self.removed_buffer_chars.iter() {
            if let Some(instance) = self.instances.get_mut_from_dustbox(&(*c).into()) {
                Self::update_instance(instance, i, &center, &self.position, &self.rotation);
            }
        }
    }

    #[inline]
    fn update_instance(
        instance: &mut GlyphInstance,
        i: &EasingPoint3,
        center: &Point2<f32>,
        position: &Point3<f32>,
        rotation: &Quaternion<f32>,
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
        instance.rotation = *rotation;
    }
}
