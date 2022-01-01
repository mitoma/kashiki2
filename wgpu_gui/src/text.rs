use cgmath::Rotation3;
use std::{collections::HashMap, sync::Arc};

use crate::{
    font_texture::{FontTexture, GlyphModel},
    model::{Instance, Instances},
};

pub struct Text {
    pub value: String,
}

pub struct GlyphInstances {
    pub glyph: Arc<GlyphModel>,
    pub instances: Instances,
}

impl Text {
    pub fn new(value: String, font_texture: &FontTexture) -> Self {
        Self {
            value: value.clone(),
        }
    }

    pub fn glyph_instances(&self, font_texture: &FontTexture) -> Vec<GlyphInstances> {
        let mut result: HashMap<char, Instances> = HashMap::new();

        let mut start_xpos: f32 = 0.0;
        let mut start_ypos: f32 = 0.0;

        let lines = self.value.lines();

        for line in lines.into_iter() {
            for c in line.chars() {
                let position = cgmath::Vector3 {
                    x: start_xpos,
                    y: start_ypos,
                    z: 0.0,
                };

                let rotation = cgmath::Quaternion::from_axis_angle(
                    cgmath::Vector3::unit_z(),
                    cgmath::Deg(0.0),
                );

                let instance = Instance::new(position, rotation);
                if let Some(instances) = result.get_mut(&c) {
                    instances.push(instance)
                } else {
                    result.insert(c, Instances::new(vec![instance]));
                }
                if let Some(glyph) = font_texture.get_glyph(c) {
                    start_xpos += glyph.width + 0.2;
                }
            }
            start_xpos = 0.0;
            start_ypos -= 1.2;
        }

        result
            .into_iter()
            .map(|(k, v)| {
                if let Some(glyph) = font_texture.get_glyph(k) {
                    GlyphInstances {
                        glyph,
                        instances: v,
                    }
                } else {
                    GlyphInstances {
                        glyph: font_texture.get_glyph('　').unwrap(),
                        instances: v,
                    }
                }
            })
            .collect()
    }
}
