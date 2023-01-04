use std::collections::BTreeMap;

use cgmath::Rotation3;
use log::debug;

use crate::{
    color_theme::ColorMode,
    font_vertex_buffer::{self, FontVertexBuffer, GlyphWidth},
    instances::{Instance, Instances},
};

pub(crate) struct SingleLineText(pub(crate) String);

impl SingleLineText {
    pub(crate) fn to_instances(
        &self,
        color_mode: ColorMode,
        font_vertex_buffer: &FontVertexBuffer,
    ) -> Vec<Instances> {
        let lines: Vec<_> = self.0.split('\n').collect();
        let max_width = lines
            .iter()
            .map(|i| {
                i.chars()
                    .map(|c| font_vertex_buffer.width(c).to_f32())
                    .sum::<f32>()
            })
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(40.0);
        let initial_x = -max_width / 2.0;
        let mut x: f32 = initial_x;
        let mut y: f32 = self.0.chars().filter(|c| *c == '\n').count() as f32 / 2.0;
        debug!("text x:{}, y:{}", x, y);
        let mut instances: BTreeMap<char, Instances> = BTreeMap::new();
        for c in self.0.chars() {
            if c == '\n' {
                x = initial_x;
                y -= 1.0;
                continue;
            }
            if x == initial_x && font_vertex_buffer.width(c) == GlyphWidth::Regular {
                x -= GlyphWidth::Regular.to_f32() / 2.0;
            }

            if !instances.contains_key(&c) {
                instances.insert(c, Instances::new(c, Vec::new()));
            }
            let instance = instances.get_mut(&c).unwrap();
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
                    x: 1.0 * x as f32,
                    y: 1.0 * y as f32,
                    z: 0.0,
                },
                cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
                color,
            );
            instance.push(i);
            x += font_vertex_buffer.width(c).to_f32();
        }
        instances.into_values().collect()
    }
}
