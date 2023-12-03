use std::{collections::BTreeMap, sync::mpsc::Receiver};

use cgmath::{Point3, Quaternion, Rotation3};
use text_buffer::{
    buffer::BufferChar,
    caret::Caret,
    editor::{ChangeEvent, Editor},
};

use crate::{
    instances::{GlyphInstance, GlyphInstances},
    layout_engine::Model,
    motion::MotionFlags,
};

pub struct TextEdit {
    editor: Editor,
    receiver: Receiver<ChangeEvent>,
    buffer_chars: BTreeMap<BufferChar, GlyphInstance>,
    removed_buffer_chars: BTreeMap<BufferChar, GlyphInstance>,
    carets: BTreeMap<Caret, GlyphInstance>,
    removed_carets: BTreeMap<Caret, GlyphInstance>,
    motion: MotionFlags,
    instances: BTreeMap<char, GlyphInstances>,
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
            instances: BTreeMap::new(),
            updated: true,
            position: Point3::new(0.0, 0.0, 0.0),
            rotation: cgmath::Quaternion::from_axis_angle(
                cgmath::Vector3::unit_y(),
                cgmath::Deg(0.0),
            ),
            bound: (0.0, 0.0),
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
        self.glyph_instances()
    }

    fn update(
        &mut self,
        color_theme: &crate::color_theme::ColorTheme,
        glyph_vertex_buffer: &mut crate::font_buffer::GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let mut changed_chars = Vec::new();
        while let Ok(event) = self.receiver.try_recv() {
            self.updated = true;
            match event {
                ChangeEvent::AddChar(c) => {
                    changed_chars.push(c);
                    self.buffer_chars.insert(c, GlyphInstance::default());
                }
                ChangeEvent::MoveChar { from, to } => {
                    if let Some(glyph_instance) = self.buffer_chars.remove(&from) {
                        changed_chars.push(to);
                        self.buffer_chars.insert(to, glyph_instance);
                    }
                }
                ChangeEvent::RemoveChar(c) => {
                    if let Some(glyph_instance) = self.buffer_chars.remove(&c) {
                        self.removed_buffer_chars.insert(c, glyph_instance);
                    }
                }
                ChangeEvent::AddCarete(c) => {
                    self.carets.insert(c, GlyphInstance::default());
                }
                ChangeEvent::MoveCarete { from, to } => {
                    if let Some(glyph_instance) = self.carets.remove(&from) {
                        self.carets.insert(to, glyph_instance);
                    }
                }
                ChangeEvent::RmoveCaret(c) => {
                    if let Some(glyph_instance) = self.carets.remove(&c) {
                        self.removed_carets.insert(c, glyph_instance);
                    }
                }
            }

            let initial_x: f32 = 0.0;
            let initial_y: f32 = 0.0;
            let current_col: usize = 0;
            let current_row: usize = 0;
            let mut x: f32 = 0.0;
            let mut y: f32 = 0.0;
            for (c, i) in self.buffer_chars.iter_mut() {
                if current_row != c.row {
                    y -= 1.0;
                    x = initial_x;
                }

                let glyph_width = glyph_vertex_buffer.width(c.c);
                x += glyph_width.left();

                let pos = cgmath::Matrix4::from(self.rotation)
                    * cgmath::Matrix4::from_translation(cgmath::Vector3 { x, y, z: 0.0 });
                let w = pos.w;
                i.position = cgmath::Vector3 {
                    x: w.x + self.position.x,
                    y: w.y + self.position.y,
                    z: w.z + self.position.z,
                };
                i.rotation = self.rotation;
            }
        }
        // むずかしいな
    }
}
