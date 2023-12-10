use std::{collections::BTreeMap, sync::mpsc::Receiver};

use cgmath::{Point3, Quaternion, Rotation3};
use log::info;
use text_buffer::{
    buffer::BufferChar,
    caret::Caret,
    editor::{ChangeEvent, Editor},
};

use crate::{
    easing_value::EasingPoint3,
    instances::{GlyphInstance, GlyphInstances},
    layout_engine::Model,
    motion::MotionFlags,
    text_instances::{TextInstances, TextInstancesKey},
};

pub struct TextEdit {
    editor: Editor,
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

impl TextEdit {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

        Self {
            editor: Editor::new(tx),
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
        while let Ok(event) = self.receiver.try_recv() {
            self.updated = true;
            match event {
                ChangeEvent::AddChar(c) => {
                    self.buffer_chars.insert(c, (0.0, 0.0, 0.0).into());
                    self.instances
                        .add(c.into(), GlyphInstance::default(), device);
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
                        position.update((0.0, 0.0, 0.0).into());
                        self.removed_buffer_chars.insert(c, position);
                    }
                }
                ChangeEvent::AddCarete(c) => {
                    self.carets
                        .insert(c, (c.row as f32, c.col as f32, 0.0).into());
                    self.instances
                        .add(c.into(), GlyphInstance::default(), device);
                }
                ChangeEvent::MoveCarete { from, to } => {
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

        if self.updated {
            let initial_x: f32 = 0.0;
            let mut current_row: usize = 0;
            let mut x: f32 = 0.0;
            let mut y: f32 = 0.0;

            for (c, i) in self.carets.iter_mut() {
                let glyph_width = glyph_vertex_buffer.width('_');
                if c.col == 0 {
                    let caret_x = initial_x + glyph_width.left();
                    let caret_y = -1.0 * c.row as f32;
                    i.update((caret_x, caret_y, 0.0).into());
                }
            }
            for (c, i) in self.buffer_chars.iter_mut() {
                if current_row != c.row {
                    let y_gain = c.row - current_row;
                    current_row = c.row;
                    y -= 1.0 * y_gain as f32;
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
                    caret_position.update((x, y, 0.0).into());
                }
            }
        }
        self.updated = false;

        {
            let rotation = self.rotation;

            // update caret
            for (c, i) in self.carets.iter() {
                if !i.in_animation() {
                    continue;
                }
                if let Some(instance) = self.instances.get_mut(&(*c).into()) {
                    let (x, y, z) = i.current();
                    let pos = cgmath::Matrix4::from(rotation)
                        * cgmath::Matrix4::from_translation(cgmath::Vector3 { x, y, z }).w;
                    let new_position = cgmath::Vector3 {
                        x: pos.x + self.position.x,
                        y: pos.y + self.position.y,
                        z: pos.z + self.position.z,
                    };
                    instance.position = new_position;
                    instance.rotation = self.rotation;
                }
            }

            // update chars
            for (c, i) in self.buffer_chars.iter() {
                if !i.in_animation() {
                    continue;
                }
                if let Some(instance) = self.instances.get_mut(&(*c).into()) {
                    let (x, y, z) = i.current();
                    let pos = cgmath::Matrix4::from(rotation)
                        * cgmath::Matrix4::from_translation(cgmath::Vector3 { x, y, z }).w;
                    let new_position = cgmath::Vector3 {
                        x: pos.x + self.position.x,
                        y: pos.y + self.position.y,
                        z: pos.z + self.position.z,
                    };
                    instance.position = new_position;
                    instance.rotation = self.rotation;
                }
            }

            // update removed chars
            for (c, i) in self.removed_buffer_chars.iter() {
                if !i.in_animation() {
                    self.instances.remove(&(*c).into());
                    continue;
                }
                if let Some(instance) = self.instances.get_mut(&(*c).into()) {
                    let (x, y, z) = i.current();
                    let pos = cgmath::Matrix4::from(rotation)
                        * cgmath::Matrix4::from_translation(cgmath::Vector3 { x, y, z }).w;
                    let new_position = cgmath::Vector3 {
                        x: pos.x + self.position.x,
                        y: pos.y + self.position.y,
                        z: pos.z + self.position.z,
                    };
                    instance.position = new_position;
                    instance.rotation = self.rotation;
                }
            }
            self.removed_buffer_chars.retain(|_c, i| i.in_animation());
        }
        self.instances.update(device, queue);
    }

    fn operation(&mut self, op: &text_buffer::action::EditorOperation) {
        self.editor.operation(op)
    }
}
