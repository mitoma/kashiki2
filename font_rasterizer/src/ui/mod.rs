use std::collections::BTreeMap;

use cgmath::{num_traits::ToPrimitive, Point3, Quaternion, Rotation3};
use instant::Duration;
use log::info;

use crate::{
    color_theme::ColorTheme,
    font_buffer::GlyphVertexBuffer,
    instances::{GlyphInstance, GlyphInstances},
    layout_engine::Model,
    motion::MotionFlags,
    time::now_millis,
};

pub struct PlaneTextReader {
    pub value: String,
    motion: MotionFlags,
    instances: BTreeMap<char, GlyphInstances>,
    updated: bool,
    position: Point3<f32>,
}

impl Model for PlaneTextReader {
    fn set_position(&mut self, position: cgmath::Point3<f32>) {
        self.position = position;
    }

    fn position(&self) -> cgmath::Point3<f32> {
        self.position
    }

    fn rotation(&self) -> cgmath::Quaternion<f32> {
        Quaternion::<f32>::new(0.0, 0.0, 0.0, 0.0)
    }

    fn length(&self) -> f32 {
        10.0
    }
}

impl PlaneTextReader {
    const MAX_WIDTH: f32 = 40.0;

    pub fn bound(&self, glyph_vertex_buffer: &GlyphVertexBuffer) -> (f32, f32) {
        let mut max_width = 0.0;
        let mut max_height = 0.0;
        for line in self.value.lines() {
            let mut width = 0.0;
            for c in line.chars() {
                if width > Self::MAX_WIDTH {
                    width = 0.0;
                    max_width = Self::MAX_WIDTH;
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

    pub fn get_target_and_camera(
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

    pub fn new(value: String) -> Self {
        Self {
            value,
            instances: BTreeMap::new(),
            updated: true,
            motion: MotionFlags::ZERO_MOTION,
            position: (0.0, 0.0, 0.0).into(),
        }
    }

    pub fn update_motion(&mut self, motion: MotionFlags) {
        self.motion = motion;
        self.updated = true;
    }

    pub fn update_value(&mut self, value: String) {
        if self.value == value {
            return;
        }
        self.value = value;
        self.updated = true;
    }

    pub fn get_instances(&self) -> Vec<&GlyphInstances> {
        self.instances.values().collect()
    }

    pub fn generate_instances(
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
        let initial_x = (-width / 2.0) + 0.5;

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
                    self.motion,
                    now_millis(),
                    0.5,
                    Duration::from_millis(1000),
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

pub struct SingleLineComponent {
    pub value: String,
    motion: MotionFlags,
    instances: BTreeMap<char, GlyphInstances>,
    updated: bool,
}

impl SingleLineComponent {
    pub fn new(value: String) -> Self {
        Self {
            value,
            instances: BTreeMap::new(),
            updated: true,
            motion: MotionFlags::ZERO_MOTION,
        }
    }

    pub fn bound(&self, glyph_vertex_buffer: &GlyphVertexBuffer) -> (f32, f32) {
        let width = self
            .value
            .chars()
            .map(|c| glyph_vertex_buffer.width(c).to_f32())
            .sum();
        (width, 1.0)
    }

    pub fn update_motion(&mut self, motion: MotionFlags) {
        self.motion = motion;
        self.updated = true;
    }

    pub fn update_value(&mut self, value: String) {
        if self.value == value {
            return;
        }
        self.value = value;
        self.updated = true;
    }

    pub fn get_instances(&self) -> Vec<&GlyphInstances> {
        self.instances.values().collect()
    }

    pub fn generate_instances(
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

        let (width, _height) = self.bound(glyph_vertex_buffer);
        let initial_x = (-width / 2.0) + 0.5;

        let mut x: f32 = initial_x;
        for c in self.value.chars() {
            let glyph_width = glyph_vertex_buffer.width(c);
            x += glyph_width.left();

            self.instances
                .entry(c)
                .or_insert_with(|| GlyphInstances::new(c, Vec::new(), device));
            let instance = self.instances.get_mut(&c).unwrap();
            let i = GlyphInstance::new(
                cgmath::Vector3 {
                    x: 0.75 * x,
                    y: 1.0,
                    z: 0.0,
                },
                cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
                get_color(color_theme, c),
                self.motion,
                now_millis(),
                0.5,
                Duration::from_millis(1000),
            );
            x += glyph_width.right();

            instance.push(i);
        }
        self.instances
            .values_mut()
            .into_iter()
            .for_each(|i| i.update_buffer(device, queue));
        self.instances.values().collect()
    }
}

enum Pos {
    First(char),
    Center(char),
    Last(char),
}

pub fn split_preedit_string(
    value: String,
    start_bytes: usize,
    end_bytes: usize,
) -> (String, String, String) {
    let splitted = value
        .chars()
        .scan(0_usize, |prev, c| {
            *prev += c.len_utf8();
            let prev = *prev;
            if prev <= start_bytes {
                Some(Pos::First(c))
            } else if prev <= end_bytes {
                Some(Pos::Center(c))
            } else {
                Some(Pos::Last(c))
            }
        })
        .collect::<Vec<_>>();
    let first: String = splitted
        .iter()
        .flat_map(|p| if let Pos::First(c) = p { Some(c) } else { None })
        .collect();
    let center: String = splitted
        .iter()
        .flat_map(|p| {
            if let Pos::Center(c) = p {
                Some(c)
            } else {
                None
            }
        })
        .collect();
    let last: String = splitted
        .iter()
        .flat_map(|p| if let Pos::Last(c) = p { Some(c) } else { None })
        .collect();
    (first, center, last)
}

#[cfg(test)]
mod test {
    use super::split_preedit_string;

    #[test]
    fn test_split1() {
        test_split("こんにちは", 6, 12, ("こん", "にち", "は"));
        test_split("こんにちは", 0, 12, ("", "こんにち", "は"));
        test_split("こんにちは", 0, 15, ("", "こんにちは", ""));
        test_split("ABCDE", 2, 3, ("AB", "C", "DE"));
        test_split("AあBいCう", 4, 8, ("Aあ", "Bい", "Cう"));
    }

    fn test_split(target: &str, start: usize, end: usize, expects: (&str, &str, &str)) {
        let (first, center, last) = split_preedit_string(target.to_string(), start, end);
        assert_eq!(&first, expects.0);
        assert_eq!(&center, expects.1);
        assert_eq!(&last, expects.2);
    }
}