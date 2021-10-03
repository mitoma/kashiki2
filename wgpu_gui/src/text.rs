use cgmath::Rotation3;
use std::{collections::HashMap, sync::Arc};

use crate::{
    font_texture::{FontTexture, GlyphModel},
    model::{Instance, Instances},
};

pub struct Text {
    value: String,
}

pub struct GlyphInstances {
    pub glyph: Arc<GlyphModel>,
    pub instances: Instances,
}

impl Text {
    pub fn new(value: String) -> Self {
        Self { value }
    }

    pub fn glyph_instances(&self, font_texture: &FontTexture) -> Vec<GlyphInstances> {
        let mut result: HashMap<char, Instances> = HashMap::new();

        let mut start_pos: f32 = 0.0;
        for c in self.value.chars() {
            let position = cgmath::Vector3 {
                x: start_pos,
                y: 0.0,
                z: 0.0,
            };
            let rotation =
                cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0));

            let instance = Instance::new(position, rotation);
            if let Some(instances) = result.get_mut(&c) {
                instances.push(instance)
            } else {
                result.insert(c, Instances::new(vec![instance]));
            }
            let glyph = font_texture.get_glyph(c).unwrap();
            start_pos += glyph.width + 0.2;
        }

        result
            .into_iter()
            .map(|(k, v)| GlyphInstances {
                glyph: font_texture.get_glyph(k).unwrap(),
                instances: v,
            })
            .collect()
    }
}
