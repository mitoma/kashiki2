mod card;
mod ime_input;
mod select_option;
mod selectbox;
mod single_line;
mod text_input;
mod textedit;
mod view_element_state;

pub use card::Card;
pub use ime_input::ImeInput;
pub use select_option::SelectOption;
pub use selectbox::SelectBox;
pub use single_line::SingleLine;
pub use text_input::TextInput;
pub use textedit::TextEdit;

use std::collections::BTreeMap;

use cgmath::{num_traits::ToPrimitive, Point3, Quaternion, Rotation3};
use instant::Duration;
use log::info;
use text_buffer::caret::CaretType;

use font_rasterizer::{
    char_width_calcurator::CharWidthCalculator,
    color_theme::ColorTheme,
    context::StateContext,
    font_buffer::Direction,
    instances::{GlyphInstance, GlyphInstances},
    motion::MotionFlags,
    time::now_millis,
};

use crate::layout_engine::{Model, ModelMode, ModelOperation, ModelOperationResult};

pub struct PlaneTextReader {
    pub value: String,
    direction: Direction,
    motion: MotionFlags,
    instances: BTreeMap<char, GlyphInstances>,
    updated: bool,
    position: Point3<f32>,
    rotation: Quaternion<f32>,
    bound: (f32, f32),
}

impl Model for PlaneTextReader {
    fn set_position(&mut self, position: cgmath::Point3<f32>) {
        if self.position == position {
            return;
        }
        self.position = position;
        self.updated = true;
    }

    fn position(&self) -> cgmath::Point3<f32> {
        self.position
    }

    fn focus_position(&self) -> Point3<f32> {
        self.position()
    }

    fn set_rotation(&mut self, rotation: Quaternion<f32>) {
        if self.rotation == rotation {
            return;
        }
        self.rotation = rotation;
        self.updated = true;
    }

    fn rotation(&self) -> cgmath::Quaternion<f32> {
        self.rotation
    }

    fn glyph_instances(&self) -> Vec<&GlyphInstances> {
        self.get_instances()
    }

    fn update(&mut self, context: &StateContext) {
        let device = &context.device;
        let queue = &context.queue;
        let color_theme = &context.color_theme;
        self.generate_instances(color_theme, &context.char_width_calcurator, device, queue);
    }

    fn bound(&self) -> (f32, f32) {
        self.bound
    }

    fn editor_operation(&mut self, _op: &text_buffer::action::EditorOperation) {
        // noop
    }

    fn model_operation(&mut self, op: &ModelOperation) -> ModelOperationResult {
        match op {
            ModelOperation::ChangeDirection(direction) => {
                self.direction = if let Some(direction) = direction {
                    *direction
                } else {
                    self.direction.toggle()
                };
                self.updated = true;
                ModelOperationResult::RequireReLayout
            }
            ModelOperation::IncreaseColInterval => ModelOperationResult::NoCare,
            ModelOperation::DecreaseColInterval => ModelOperationResult::NoCare,
            ModelOperation::IncreaseRowInterval => ModelOperationResult::NoCare,
            ModelOperation::DecreaseRowInterval => ModelOperationResult::NoCare,
            ModelOperation::IncreaseRowScale => ModelOperationResult::NoCare,
            ModelOperation::DecreaseRowScale => ModelOperationResult::NoCare,
            ModelOperation::IncreaseColScale => ModelOperationResult::NoCare,
            ModelOperation::DecreaseColScale => ModelOperationResult::NoCare,
            ModelOperation::CopyDisplayString(_, _) => ModelOperationResult::NoCare,
            ModelOperation::TogglePsychedelic => ModelOperationResult::NoCare,
            ModelOperation::MoveToClick(_, _, _) => ModelOperationResult::NoCare,
            ModelOperation::MarkAndClick(_, _, _) => ModelOperationResult::NoCare,
            ModelOperation::ToggleMinBound => ModelOperationResult::NoCare,
        }
    }

    fn to_string(&self) -> String {
        self.value.clone()
    }

    fn model_mode(&self) -> ModelMode {
        ModelMode::Nomal
    }

    fn in_animation(&self) -> bool {
        false
    }
}

impl PlaneTextReader {
    const MAX_WIDTH: f32 = 40.0;

    fn calc_bound(&self, char_width_calcurator: &CharWidthCalculator) -> (f32, f32) {
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
                let char_width = char_width_calcurator.get_width(c);
                width += char_width.to_f32();
            }
            max_height += 1.0;
            if width > max_width {
                max_width = width;
            }
        }
        (max_width, max_height)
    }

    #[allow(unused)]
    fn get_target_and_camera(
        &self,
        line_num: usize,
        char_width_calcurator: &CharWidthCalculator,
    ) -> (cgmath::Point3<f32>, cgmath::Point3<f32>, usize) {
        let line_num = (line_num as f32).min(self.calc_bound(char_width_calcurator).1);
        (
            (0.0, -line_num, 0.0).into(),
            (0.0, -line_num, 50.0).into(),
            line_num.to_usize().unwrap_or_default(),
        )
    }

    pub fn new(value: String) -> Self {
        Self {
            value,
            direction: Direction::Horizontal,
            instances: BTreeMap::new(),
            updated: true,
            motion: MotionFlags::ZERO_MOTION,
            position: (0.0, 0.0, 0.0).into(),
            rotation: cgmath::Quaternion::from_axis_angle(
                cgmath::Vector3::unit_y(),
                cgmath::Deg(0.0),
            ),
            bound: (0.0, 0.0),
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
        color_theme: &ColorTheme,
        char_width_calcurator: &CharWidthCalculator,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Vec<&GlyphInstances> {
        if !self.updated {
            return self.instances.values().collect();
        }

        self.instances.clear();
        self.updated = false;

        let lines: Vec<_> = self.value.split('\n').collect();

        let (width, height) = self.calc_bound(char_width_calcurator);
        self.bound = (width, height);
        let initial_x = (-width / 2.0) + 0.5;
        let initial_y = (height / 2.0) - 0.5;

        let mut x: f32 = initial_x;
        let mut y: f32 = initial_y;
        let rotation = self.rotation();
        for line in lines {
            for c in line.chars() {
                if x > width / 2.0 {
                    x = initial_x;
                    y -= 1.0;
                }
                let char_width = char_width_calcurator.get_width(c);
                x += char_width.left();

                self.instances
                    .entry(c)
                    .or_insert_with(|| GlyphInstances::new(c, device));
                let instance = self.instances.get_mut(&c).unwrap();
                //let pos = cgmath::Matrix4::from(rotation)
                //    * cgmath::Matrix4::from_translation(cgmath::Vector3 {
                //        x: 0.75 * x + self.position.x,
                //        y: 1.0 * y + self.position.y,
                //        z: 0.0 + self.position.z,
                //    });
                let pos = cgmath::Matrix4::from(rotation)
                    * cgmath::Matrix4::from_translation(cgmath::Vector3 {
                        x: 1.0 * x,
                        y: 1.0 * y,
                        z: 0.0,
                    });
                let w = pos.w;
                let i = GlyphInstance::new(
                    cgmath::Vector3 {
                        x: w.x + self.position.x,
                        y: w.y + self.position.y,
                        z: w.z + self.position.z,
                    },
                    rotation,
                    //cgmath::Quaternion::from_axis_angle(
                    //    cgmath::Vector3::unit_z(),
                    //    cgmath::Deg(0.0),
                    //),
                    [1.0, 1.0],
                    [1.0, 1.0],
                    get_color(color_theme, c),
                    self.motion,
                    now_millis(),
                    0.5,
                    Duration::from_millis(1000),
                );
                x += char_width.right();

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

        self.instances.values_mut().for_each(|i| {
            i.set_direction(&self.direction);
            i.update_buffer(device, queue);
        });

        self.instances.values().collect()
    }
}

fn get_color(color_theme: &ColorTheme, c: char) -> [f32; 3] {
    if c.is_ascii() {
        color_theme.yellow().get_color()
    } else if ('あ'..'一').contains(&c) {
        color_theme.text().get_color()
    } else if c < '\u{1F600}' {
        color_theme.cyan().get_color()
    } else {
        color_theme.green().get_color()
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

#[inline]
pub fn caret_char(caret_type: CaretType) -> char {
    match caret_type {
        CaretType::Primary => '_',
        CaretType::Mark => '^',
    }
}

#[inline]
pub fn ime_chars() -> [char; 2] {
    ['[', ']']
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
