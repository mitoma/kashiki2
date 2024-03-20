use std::collections::BTreeMap;

use cgmath::{Point2, Point3, Quaternion, Rotation3};
use instant::Duration;
use rand::Rng;
use text_buffer::buffer::BufferChar;
use wgpu::Device;

use crate::{
    char_width_calcurator::CharWidth,
    color_theme::ColorTheme,
    easing_value::EasingPointN,
    font_buffer::{Direction, GlyphVertexBuffer},
    instances::GlyphInstance,
    motion::MotionFlags,
    text_instances::TextInstances,
    time::now_millis,
};

use super::textedit::TextEditConfig;

struct ViewElementState {
    position: EasingPointN<3>,
    color: EasingPointN<3>,
}

#[derive(Default)]
pub(crate) struct ViewCharStates {
    default_motion: MotionFlags,
    chars: BTreeMap<BufferChar, ViewElementState>,
    removed_chars: BTreeMap<BufferChar, ViewElementState>,
    pub(crate) instances: TextInstances,
}

impl ViewCharStates {
    pub(crate) fn add_char(
        &mut self,
        c: BufferChar,
        position: [f32; 3],
        color: [f32; 3],
        device: &Device,
    ) {
        let mut easing_color = EasingPointN::new(color);
        easing_color.update_duration_and_easing_func(
            Duration::from_millis(800),
            nenobi::functions::sin_in_out,
        );
        let state = ViewElementState {
            position: EasingPointN::new(position),
            color: easing_color,
        };
        self.chars.insert(c, state);
        let instance = GlyphInstance {
            color,
            motion: self.default_motion,
            ..GlyphInstance::default()
        };
        self.instances.add(c.into(), instance, device);
    }

    pub(crate) fn update_char(&mut self, from: BufferChar, to: BufferChar, device: &Device) {
        if let Some(position) = self.chars.remove(&from) {
            self.chars.insert(to, position);
        }
        if let Some(instance) = self.instances.remove(&from.into()) {
            self.instances.add(to.into(), instance, device);
        }
    }

    pub(crate) fn update_position(&mut self, c: &BufferChar, position: [f32; 3]) {
        if let Some(c_pos) = self.chars.get_mut(c) {
            c_pos.position.update(position);
        }
    }

    pub(crate) fn update_char_theme(&mut self, color_theme: &ColorTheme) {
        self.chars.iter_mut().for_each(|(_, i)| {
            i.color.update(color_theme.text().get_color());
        });
    }

    pub(crate) fn update_instances(
        &mut self,
        update_environment: bool,
        center: &Point2<f32>,
        position: &Point3<f32>,
        rotation: &Quaternion<f32>,
        glyph_vertex_buffer: &GlyphVertexBuffer,
        text_edit_config: &TextEditConfig,
    ) {
        // update chars
        for (c, i) in self.chars.iter_mut() {
            if !update_environment && !i.position.in_animation() && !i.color.in_animation() {
                continue;
            }
            if let Some(instance) = self.instances.get_mut(&(*c).into()) {
                let char_rotation = Self::calc_rotation(c.c, text_edit_config, glyph_vertex_buffer);
                Self::update_instance(instance, i, center, position, rotation, char_rotation);
            }
        }

        // update removed chars
        self.clean_dustbox();
        for (c, i) in self.removed_chars.iter_mut() {
            if !update_environment && !i.position.in_animation() && !i.color.in_animation() {
                continue;
            }
            if let Some(instance) = self.instances.get_mut_from_dustbox(&(*c).into()) {
                let char_rotation = Self::calc_rotation(c.c, text_edit_config, glyph_vertex_buffer);
                Self::update_instance(instance, i, center, position, rotation, char_rotation);
            }
        }
    }

    fn update_instance(
        instance: &mut GlyphInstance,
        view_char_state: &mut ViewElementState,
        center: &Point2<f32>,
        position: &Point3<f32>,
        rotation: &Quaternion<f32>,
        char_rotation: Option<Quaternion<f32>>,
    ) {
        // set color
        instance.color = view_char_state.color.current();

        // set position
        let [x, y, z] = view_char_state.position.current();
        let pos = cgmath::Matrix4::from(*rotation)
            * cgmath::Matrix4::from_translation(cgmath::Vector3 { x, y, z }).w;
        let new_position = cgmath::Vector3 {
            x: pos.x - center.x + position.x,
            y: pos.y - center.y + position.y,
            z: pos.z + position.z,
        };
        instance.position = new_position;

        // set rotation
        // 縦書きの場合は char_rotation が必要なのでここで回転する
        instance.rotation = match char_rotation {
            Some(r) => *rotation * r,
            None => *rotation,
        }
    }

    #[inline]
    fn calc_rotation(
        c: char,
        text_edit_config: &TextEditConfig,
        glyph_vertex_buffer: &GlyphVertexBuffer,
    ) -> Option<Quaternion<f32>> {
        match text_edit_config.direction {
            Direction::Horizontal => None,
            Direction::Vertical => {
                let width = glyph_vertex_buffer.width(c);
                match width {
                    CharWidth::Regular => Some(cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_z(),
                        cgmath::Deg(-90.0),
                    )),
                    CharWidth::Wide => None,
                }
            }
        }
    }

    // BufferChar をゴミ箱に移動する(削除モーションに入る)
    pub(crate) fn char_to_dustbox(&mut self, c: BufferChar) {
        if let Some(mut state) = self.chars.remove(&c) {
            state.position.add((0.0, -1.0, 0.0).into());
            self.removed_chars.insert(c, state);
        }
        self.instances.pre_remove(&c.into());
    }

    // ゴミ箱の文字の削除モーションが完了しているものを削除する
    fn clean_dustbox(&mut self) {
        self.removed_chars.retain(|c, i| {
            let in_animation = i.position.in_animation();
            // こいつは消えゆく運命の文字なので position_updated なんて考慮せずに in_animation だけ見る
            if !in_animation {
                self.instances.remove_from_dustbox(&(*c).into());
            }
            in_animation
        });
    }

    pub(crate) fn set_motion_and_color(&mut self, text_edit_config: &TextEditConfig) {
        if text_edit_config.psychedelic {
            for (c, i) in self.chars.iter_mut() {
                if let Some(instance) = self.instances.get_mut(&(*c).into()) {
                    instance.motion = MotionFlags::random_motion();
                    instance.start_time = now_millis();
                    instance.duration =
                        Duration::from_millis(rand::thread_rng().gen_range(300..3000));
                    instance.gain = rand::thread_rng().gen_range(0.1..1.0);
                    i.color.update(match rand::thread_rng().gen_range(0..8) {
                        0 => text_edit_config.color_theme.yellow().get_color(),
                        1 => text_edit_config.color_theme.orange().get_color(),
                        2 => text_edit_config.color_theme.red().get_color(),
                        3 => text_edit_config.color_theme.magenta().get_color(),
                        4 => text_edit_config.color_theme.violet().get_color(),
                        5 => text_edit_config.color_theme.blue().get_color(),
                        6 => text_edit_config.color_theme.cyan().get_color(),
                        7 => text_edit_config.color_theme.green().get_color(),
                        _ => text_edit_config.color_theme.text().get_color(),
                    });
                }
            }
        } else {
            for (c, i) in self.chars.iter_mut() {
                if let Some(instance) = self.instances.get_mut(&(*c).into()) {
                    instance.motion = self.default_motion;
                    instance.start_time = now_millis();
                    instance.duration = Duration::ZERO;
                    instance.gain = rand::thread_rng().gen_range(0.1..1.0);
                    i.color
                        .update(text_edit_config.color_theme.text().get_color());
                }
            }
        }
    }
}
