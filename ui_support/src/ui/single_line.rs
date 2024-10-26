use std::collections::BTreeMap;

use cgmath::Rotation3;
use instant::Duration;

use font_rasterizer::{
    char_width_calcurator::CharWidthCalculator,
    context::StateContext,
    instances::{GlyphInstance, GlyphInstances},
    motion::MotionFlags,
    time::now_millis,
};

use super::get_color;

pub struct SingleLine {
    pub value: String,
    motion: MotionFlags,
    instances: BTreeMap<char, GlyphInstances>,
    updated: bool,
    scale: [f32; 2],
    width: Option<f32>,
}

impl SingleLine {
    pub fn new(value: String) -> Self {
        Self {
            value,
            instances: BTreeMap::new(),
            updated: true,
            motion: MotionFlags::ZERO_MOTION,
            scale: [1.0, 1.0],
            width: None,
        }
    }

    fn bound(&self, char_width_calcurator: &CharWidthCalculator) -> (f32, f32) {
        let width = self
            .value
            .chars()
            .map(|c| char_width_calcurator.get_width(c).to_f32())
            .sum();
        (width, 1.0)
    }

    pub fn update_motion(&mut self, motion: MotionFlags) {
        self.motion = motion;
        self.updated = true;
    }

    pub fn update_width(&mut self, width: Option<f32>) {
        self.width = width;
        self.updated = true;
    }

    pub fn update_value(&mut self, value: String) {
        if self.value == value {
            return;
        }
        self.value = value;
        self.updated = true;
    }

    pub fn update_scale(&mut self, scale: [f32; 2]) {
        if self.scale == scale {
            return;
        }
        self.scale = scale;
        self.updated = true;
    }

    pub fn get_instances(&self) -> Vec<&GlyphInstances> {
        self.instances.values().collect()
    }

    pub fn generate_instances(&mut self, context: &StateContext) -> Vec<&GlyphInstances> {
        if !self.updated {
            return self.instances.values().collect();
        }

        self.instances.clear();
        self.updated = false;

        let (width, _height) = self.bound(&context.char_width_calcurator);
        let initial_x = (-width / 2.0) + 0.5;

        // 横幅が固定の時にはスケールを変更して画面内に収まるように心がける
        let mut x_scale = self.scale[0];
        if let Some(fixed_width) = self.width {
            // 画面のアスペクト比を考慮して横幅を調整
            let fixed_width = fixed_width * context.window_size.aspect();
            if x_scale > fixed_width / width {
                x_scale = fixed_width / width;
            }
        }

        let mut x: f32 = initial_x;
        let y_pos = -0.3 / self.scale[1];
        for c in self.value.chars() {
            let char_width = context.char_width_calcurator.get_width(c);
            x += char_width.left();

            self.instances
                .entry(c)
                .or_insert_with(|| GlyphInstances::new(c, &context.device));
            let instance = self.instances.get_mut(&c).unwrap();
            let i = GlyphInstance::new(
                cgmath::Vector3 {
                    x: x * 0.7,
                    y: y_pos,
                    z: 0.0,
                },
                cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
                [x_scale, self.scale[1]],
                [1.0, 1.0],
                get_color(&context.color_theme, c),
                self.motion,
                now_millis(),
                0.5,
                Duration::from_millis(300),
            );
            x += char_width.right();

            instance.push(i);
        }
        self.instances
            .values_mut()
            .for_each(|i| i.update_buffer(&context.device, &context.queue));
        self.instances.values().collect()
    }
}
