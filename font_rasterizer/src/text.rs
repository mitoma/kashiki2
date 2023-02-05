use std::collections::BTreeMap;

use anyhow::Context;
use cgmath::{num_traits::ToPrimitive, Rotation3};
use instant::Duration;
use log::{debug, info};

use crate::{
    color_theme::ColorTheme,
    font_buffer::GlyphVertexBuffer,
    instances::{GlyphInstance, GlyphInstances},
    motion::MotionFlags,
    time::now_millis,
};

pub(crate) struct MultiLineText {
    value: String,
    instances: BTreeMap<char, GlyphInstances>,
    updated: bool,
}

impl MultiLineText {
    pub(crate) fn new(value: String) -> Self {
        Self {
            value,
            instances: BTreeMap::new(),
            updated: true,
        }
    }

    pub(crate) fn update_value(&mut self, value: String) {
        self.value = value;
        self.updated = true;
    }

    pub(crate) fn get_target(&self, count: usize) -> anyhow::Result<cgmath::Vector3<f32>> {
        let mut target_char = self
            .value
            .chars()
            .into_iter()
            .nth(count)
            .with_context(|| "get nth char")?;
        if !self.instances.contains_key(&target_char) {
            target_char = self.value.chars().next().unwrap();
        }
        let instance_count = self
            .value
            .chars()
            .into_iter()
            .take(count)
            .filter(|c| *c == target_char)
            .count();
        Ok(self.instances[&target_char].nth_position(instance_count))
    }

    pub(crate) fn generate_instances(
        &mut self,
        color_theme: ColorTheme,
        glyph_vertex_buffer: &GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Vec<&GlyphInstances> {
        if !self.updated {
            return self.instances.values().collect();
        }
        self.instances.clear();
        self.updated = false;

        let lines: Vec<_> = self.value.split('\n').collect();
        let width = lines
            .iter()
            .map(|i| {
                i.chars()
                    .map(|c| glyph_vertex_buffer.width(c).to_f32())
                    .sum::<f32>()
            })
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(40.0);
        let width = if width > 40.0 { 40.0 } else { width };

        let height = self.value.chars().filter(|c| *c == '\n').count() as f32;
        let initial_x = -width / 2.0;
        let mut x: f32 = initial_x;
        let mut y: f32 = height / 2.0;
        debug!("text x:{}, y:{}", x, y);
        for c in self.value.chars() {
            if c == '\n' {
                x = initial_x;
                y -= 1.0;
                continue;
            }
            if x > width / 2.0 {
                x = initial_x;
                y -= 1.0;
            }

            let glyph_width = glyph_vertex_buffer.width(c);
            x += glyph_width.left();

            self.instances
                .entry(c)
                .or_insert_with(|| GlyphInstances::new(c, Vec::new(), device));
            let instance = self.instances.get_mut(&c).unwrap();
            let i = GlyphInstance::new(
                cgmath::Vector3 {
                    x: 0.75 * x,
                    y: 1.0 * y,
                    z: 0.0,
                },
                cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
                get_color(color_theme, c),
                MotionFlags::ZERO_MOTION,
                now_millis(),
                0.0,
                Duration::from_millis(0),
            );
            instance.push(i);
            x += glyph_width.right();
        }
        self.instances
            .values_mut()
            .into_iter()
            .for_each(|i| i.update_buffer(device, queue));

        self.instances.values().collect()
    }
}

pub(crate) struct PlaneTextReader {
    value: String,
    instances: BTreeMap<char, GlyphInstances>,
    updated: bool,
}

impl PlaneTextReader {
    const MAX_WIDTH: f32 = 40.0;

    fn bound(&self, glyph_vertex_buffer: &GlyphVertexBuffer) -> (f32, f32) {
        let mut max_width = 0.0;
        let mut max_height = 0.0;
        for line in self.value.lines() {
            let mut width = 0.0;
            for c in line.chars() {
                if width > Self::MAX_WIDTH {
                    width = 0.0;
                    max_height += 1.0;
                }
                let glyph_width = glyph_vertex_buffer.width(c);
                width += glyph_width.to_f32();
            }
            max_height += 1.0;
            if width > max_width {
                max_width = width;
            }
        }
        (max_width, max_height)
    }

    pub(crate) fn get_target_and_camera(
        &self,
        line_num: usize,
        glyph_vertex_buffer: &GlyphVertexBuffer,
    ) -> anyhow::Result<(cgmath::Point3<f32>, cgmath::Point3<f32>, usize)> {
        let line_num = (line_num as f32).min(self.bound(glyph_vertex_buffer).1);
        Ok((
            (0.0, -line_num, 0.0).into(),
            (0.0, -line_num, 50.0).into(),
            line_num.to_usize().unwrap_or_default(),
        ))
    }

    pub(crate) fn new(value: String) -> Self {
        Self {
            value,
            instances: BTreeMap::new(),
            updated: true,
        }
    }

    pub(crate) fn update_value(&mut self, value: String) {
        self.value = value;
        self.updated = true;
    }

    pub(crate) fn generate_instances(
        &mut self,
        color_theme: ColorTheme,
        glyph_vertex_buffer: &GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Vec<&GlyphInstances> {
        if !self.updated {
            return self.instances.values().collect();
        }

        self.instances.clear();
        self.updated = false;

        let lines: Vec<_> = self.value.split('\n').collect();

        let (width, height) = self.bound(glyph_vertex_buffer);
        let initial_x = -width / 2.0;

        let mut x: f32 = initial_x;
        let mut y: f32 = 0.0;
        for line in lines {
            for c in line.chars() {
                if x > width / 2.0 {
                    x = initial_x;
                    y -= 1.0;
                }
                let glyph_width = glyph_vertex_buffer.width(c);
                x += glyph_width.left();

                self.instances
                    .entry(c)
                    .or_insert_with(|| GlyphInstances::new(c, Vec::new(), device));
                let instance = self.instances.get_mut(&c).unwrap();
                let i = GlyphInstance::new(
                    cgmath::Vector3 {
                        x: 0.75 * x,
                        y: 1.0 * y,
                        z: 0.01 * x * x,
                    },
                    cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_z(),
                        cgmath::Deg(0.0),
                    ),
                    get_color(color_theme, c),
                    MotionFlags::ZERO_MOTION,
                    now_millis(),
                    0.0,
                    Duration::from_millis(0),
                );
                x += glyph_width.right();

                instance.push(i);
            }
            x = initial_x;
            y -= 1.0;
        }
        info!(
            "height:{}, last_y:{}, instance_count:{}",
            height,
            y,
            self.instances.values().map(|i| i.len()).sum::<usize>()
        );

        self.instances
            .values_mut()
            .into_iter()
            .for_each(|i| i.update_buffer(device, queue));

        self.instances.values().collect()
    }
}

fn get_color(color_theme: ColorTheme, c: char) -> [f32; 3] {
    if c.is_ascii() {
        color_theme.yellow().get_color()
    } else if ('あ'..'一').contains(&c) {
        color_theme.text().get_color()
    } else if c < '\u{1F600}' {
        color_theme.text_emphasized().get_color()
    } else {
        color_theme.text_comment().get_color()
    }
}
