use std::collections::BTreeMap;

use cgmath::Rotation3;
use log::debug;

use crate::{
    instances::{Instance, Instances},
    SolarizedColor,
};

pub(crate) struct SingleLineText(pub(crate) String);

impl SingleLineText {
    pub(crate) fn to_instances(&self) -> Vec<Instances> {
        let ix: Vec<_> = self.0.split('\n').collect();
        let max_length = ix.iter().map(|i| i.chars().count()).max().unwrap();
        let initial_x = -(max_length as i32 / 2);
        let mut x: i32 = initial_x;
        let mut y: i32 = self.0.chars().filter(|c| *c == '\n').count() as i32 / 2;
        debug!("text x:{}, y:{}", x, y);
        let mut instances: BTreeMap<char, Instances> = BTreeMap::new();
        for c in self.0.chars() {
            if c == '\n' {
                x = initial_x;
                y -= 1;
                continue;
            }
            if !instances.contains_key(&c) {
                instances.insert(c, Instances::new(c, Vec::new()));
            }
            let instance = instances.get_mut(&c).unwrap();

            let color = if c.is_ascii() {
                SolarizedColor::BASE0.get_color()
            } else if c < '一' {
                SolarizedColor::GREEN.get_color()
            } else if c < '\u{1F600}' {
                SolarizedColor::VIOLET.get_color()
            } else {
                SolarizedColor::YELLOW.get_color()
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
            x += 1;
        }
        instances.into_values().collect()
    }
}
