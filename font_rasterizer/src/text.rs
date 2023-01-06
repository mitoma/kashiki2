use std::collections::BTreeMap;

use cgmath::Rotation3;
use log::debug;

use crate::{
    color_theme::ColorMode,
    font_vertex_buffer::FontVertexBuffer,
    instances::{Instance, Instances},
};

pub(crate) struct SingleLineText(String, BTreeMap<char, Instances>, bool);

impl SingleLineText {
    pub(crate) fn new(value: String) -> Self {
        Self(value, BTreeMap::new(), true)
    }

    pub(crate) fn generate_instances(
        &mut self,
        color_mode: ColorMode,
        font_vertex_buffer: &FontVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Vec<&Instances> {
        if !self.2 {
            return self.1.values().collect();
        }
        self.2 = false;

        let lines: Vec<_> = self.0.split('\n').collect();
        let width = lines
            .iter()
            .map(|i| {
                i.chars()
                    .map(|c| font_vertex_buffer.width(c).to_f32())
                    .sum::<f32>()
            })
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(40.0);
        let width = if width > 40.0 { 40.0 } else { width };

        let height = self.0.chars().filter(|c| *c == '\n').count() as f32;
        let initial_x = -width / 2.0;
        let mut x: f32 = initial_x;
        let mut y: f32 = height / 2.0;
        debug!("text x:{}, y:{}", x, y);
        for c in self.0.chars() {
            if c == '\n' {
                x = initial_x;
                y -= 1.0;
                continue;
            }
            if x > width / 2.0 {
                x = initial_x;
                y -= 1.0;
            }

            let glyph_width = font_vertex_buffer.width(c);
            x += glyph_width.left();

            self.1
                .entry(c)
                .or_insert_with(|| Instances::new(c, Vec::new(), device));
            let instance = self.1.get_mut(&c).unwrap();
            let color = if c.is_ascii() {
                color_mode.yellow().get_color()
            } else if c < 'あ' {
                color_mode.text().get_color()
            } else if c < '\u{1F600}' {
                color_mode.text_comment().get_color()
            } else {
                color_mode.text_emphasized().get_color()
            };
            let i = Instance::new(
                cgmath::Vector3 {
                    x: 0.75 * x,
                    y: 1.0 * y,
                    z: 0.0,
                },
                cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
                color,
            );
            instance.push(i);
            x += glyph_width.right();
        }
        self.1
            .values_mut()
            .into_iter()
            .for_each(|i| i.update_buffer(queue));

        self.1.values().collect()
    }
}
