use std::collections::BTreeMap;

use cgmath::{Point2, Point3, Quaternion, Rotation3};
use instant::Duration;
use log::debug;
use rand::Rng;
use text_buffer::{
    buffer::BufferChar,
    caret::{Caret, CaretType},
};
use wgpu::Device;

use crate::{
    char_width_calcurator::CharWidth,
    color_theme::{ColorTheme, ThemedColor},
    easing_value::EasingPointN,
    font_buffer::{Direction, GlyphVertexBuffer},
    instances::GlyphInstance,
    motion::MotionFlags,
    text_instances::TextInstances,
    time::now_millis,
};

use super::{caret_char, textedit::TextEditConfig};

struct ViewElementState {
    pub(crate) base_color: ThemedColor,
    pub(crate) in_selection: bool,
    pub(crate) position: EasingPointN<3>,
    pub(crate) color: EasingPointN<3>,
    pub(crate) scale: EasingPointN<2>,
    pub(crate) motion_gain: EasingPointN<1>,
}

impl ViewElementState {
    pub(crate) fn in_animation(&mut self) -> bool {
        let position_animation = self.position.in_animation();
        let color_animation = self.color.in_animation();
        let scale_animation = self.scale.in_animation();
        let motion_gain_animation = self.motion_gain.in_animation();
        position_animation | color_animation | scale_animation | motion_gain_animation
    }
}

#[derive(Default)]
pub(crate) struct CharStates {
    default_motion: MotionFlags,
    chars: BTreeMap<BufferChar, ViewElementState>,
    removed_chars: BTreeMap<BufferChar, ViewElementState>,
    pub(crate) instances: TextInstances,
}

impl CharStates {
    pub(crate) fn add_char(
        &mut self,
        c: BufferChar,
        position: [f32; 3],
        color: [f32; 3],
        config: &TextEditConfig,
        device: &Device,
    ) {
        let mut easing_color = EasingPointN::new(color);
        easing_color.update_duration_and_easing_func(
            Duration::from_millis(800),
            nenobi::functions::sin_in_out,
        );
        let state = ViewElementState {
            position: EasingPointN::new(position),
            in_selection: false,
            base_color: ThemedColor::Text,
            color: easing_color,
            scale: EasingPointN::new([1.0, 1.0]),
            motion_gain: EasingPointN::new([config.char_easings.add_char.gain]),
        };
        self.chars.insert(c, state);
        let instance = GlyphInstance {
            color,
            start_time: now_millis(),
            motion: config.char_easings.add_char.motion,
            duration: config.char_easings.add_char.duration,
            ..GlyphInstance::default()
        };
        self.instances.add(c.into(), instance, device);
    }

    pub(crate) fn move_char(
        &mut self,
        from: BufferChar,
        to: BufferChar,
        config: &TextEditConfig,
        device: &Device,
    ) {
        if let Some(mut position) = self.chars.remove(&from) {
            position
                .motion_gain
                .update([config.char_easings.move_char.gain]);
            self.chars.insert(to, position);
        }
        if let Some(mut instance) = self.instances.remove(&from.into()) {
            instance.start_time = now_millis();
            instance.motion = config.char_easings.move_char.motion;
            instance.duration = config.char_easings.move_char.duration;
            self.instances.add(to.into(), instance, device);
        }
    }

    pub(crate) fn update_state_position(&mut self, c: &BufferChar, position: [f32; 3]) {
        if let Some(c_pos) = self.chars.get_mut(c) {
            c_pos.position.update(position);
        }
    }

    pub(crate) fn update_char_theme(&mut self, color_theme: &ColorTheme) {
        self.chars.iter_mut().for_each(|(_, i)| {
            let color = if i.in_selection {
                i.base_color.get_selection_color(color_theme)
            } else {
                i.base_color.get_color(color_theme)
            };
            i.color.update(color);
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
            if !update_environment && !i.in_animation() {
                continue;
            }
            if let Some(instance) = self.instances.get_mut(&(*c).into()) {
                let char_rotation = calc_rotation(c.c, text_edit_config, glyph_vertex_buffer);
                update_instance(instance, i, center, position, rotation, char_rotation);
            }
        }

        // update removed chars
        self.clean_dustbox();
        for (c, i) in self.removed_chars.iter_mut() {
            if !update_environment && !i.in_animation() {
                continue;
            }
            if let Some(instance) = self.instances.get_mut_from_dustbox(&(*c).into()) {
                let char_rotation = calc_rotation(c.c, text_edit_config, glyph_vertex_buffer);
                update_instance(instance, i, center, position, rotation, char_rotation);
            }
        }
    }

    // BufferChar をゴミ箱に移動する(削除モーションに入る)
    pub(crate) fn char_to_dustbox(&mut self, c: BufferChar, config: &TextEditConfig) {
        if let Some(mut state) = self.chars.remove(&c) {
            // アニメーション状態に強制的に有効にするために gain を 0 にしている。
            // 本当はアニメーションが終わったらゴミ箱から消すという仕様が適切ではないのかもしれない
            state.motion_gain.update([0.0]);
            state
                .motion_gain
                .update([config.char_easings.remove_char.gain]);
            self.removed_chars.insert(c, state);
        }
        self.instances.get_mut(&c.into()).map(|instance| {
            instance.start_time = now_millis();
            instance.motion = config.char_easings.remove_char.motion;
            instance.duration = config.char_easings.remove_char.duration;
        });
        self.instances.pre_remove(&c.into());
    }

    // ゴミ箱の文字の削除モーションが完了しているものを削除する
    fn clean_dustbox(&mut self) {
        self.removed_chars.retain(|c, i| {
            let in_animation = i.in_animation();
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
                    //    instance.gain = rand::thread_rng().gen_range(0.1..1.0);
                    i.motion_gain
                        .update([rand::thread_rng().gen_range(0.1..1.0)]);
                    i.base_color = match rand::thread_rng().gen_range(0..8) {
                        0 => ThemedColor::Yellow,
                        1 => ThemedColor::Orange,
                        2 => ThemedColor::Red,
                        3 => ThemedColor::Magenta,
                        4 => ThemedColor::Violet,
                        5 => ThemedColor::Blue,
                        6 => ThemedColor::Cyan,
                        7 => ThemedColor::Green,
                        _ => ThemedColor::Text,
                    };
                    i.color
                        .update(i.base_color.get_color(&text_edit_config.color_theme));
                }
            }
        } else {
            for (_, i) in self.chars.iter_mut() {
                i.motion_gain.update([0.0]);
                i.base_color = ThemedColor::Text;
                i.color
                    .update(i.base_color.get_color(&text_edit_config.color_theme));
            }
        }
    }

    pub(crate) fn select_char(&mut self, c: BufferChar, text_edit_config: &TextEditConfig) {
        debug!("select_char: {:?}", c);
        let _ = self.chars.get_mut(&c).map(|state| {
            state.in_selection = true;
            state.color.update(
                state
                    .base_color
                    .get_selection_color(&text_edit_config.color_theme),
            );
        });
    }

    pub(crate) fn unselect_char(&mut self, c: BufferChar, text_edit_config: &TextEditConfig) {
        debug!("unselect_char: {:?}", c);
        let _ = self.chars.get_mut(&c).map(|state| {
            state.in_selection = false;
            state
                .color
                .update(state.base_color.get_color(&text_edit_config.color_theme));
        });
    }
}

#[derive(Default)]
pub(crate) struct CaretStates {
    pub(crate) default_motion: MotionFlags,
    main_caret: Option<(Caret, ViewElementState)>,
    mark: Option<(Caret, ViewElementState)>,
    removed_carets: BTreeMap<Caret, ViewElementState>,
    pub(crate) instances: TextInstances,
}

impl CaretStates {
    pub(crate) fn main_caret_position(&self) -> Option<[f32; 3]> {
        self.main_caret.as_ref().map(|(_, s)| s.position.last())
    }

    pub(crate) fn add_caret(&mut self, c: Caret, color: [f32; 3], device: &Device) {
        let position = [c.row as f32, c.col as f32, 0.0];

        let mut easing_color = EasingPointN::new(color);
        easing_color.update_duration_and_easing_func(
            Duration::from_millis(800),
            nenobi::functions::sin_in_out,
        );
        let state = ViewElementState {
            position: EasingPointN::new(position),
            in_selection: false,
            base_color: ThemedColor::TextEmphasized,
            color: easing_color,
            scale: EasingPointN::new([1.0, 1.0]),
            motion_gain: EasingPointN::new([0.0]),
        };
        if c.caret_type == CaretType::Primary {
            self.main_caret.replace((c, state));
        } else {
            self.mark.replace((c, state));
        }

        let caret_instance = GlyphInstance {
            color,
            motion: self.default_motion,
            ..GlyphInstance::default()
        };
        self.instances.add(c.into(), caret_instance, device);
    }

    pub(crate) fn move_caret(&mut self, from: Caret, to: Caret, device: &Device) {
        match from.caret_type {
            CaretType::Primary => self.main_caret = Some((to, self.main_caret.take().unwrap().1)),
            CaretType::Mark => self.mark = Some((to, self.mark.take().unwrap().1)),
        }
        if let Some(instance) = self.instances.remove(&from.into()) {
            self.instances.add(to.into(), instance, device);
        }
    }

    // BufferChar をゴミ箱に移動する(削除モーションに入る)
    pub(crate) fn caret_to_dustbox(&mut self, c: Caret) {
        match c.caret_type {
            CaretType::Primary => {
                if let Some((_, mut state)) = self.main_caret.take() {
                    state.position.add((0.0, -1.0, 0.0).into());
                    self.removed_carets.insert(c, state);
                }
            }
            CaretType::Mark => {
                if let Some((_, mut state)) = self.mark.take() {
                    state.position.add((0.0, -1.0, 0.0).into());
                    self.removed_carets.insert(c, state);
                }
            }
        }
        self.instances.pre_remove(&c.into());
    }

    pub(crate) fn update_state_position(&mut self, caret_type: CaretType, position: [f32; 3]) {
        match caret_type {
            CaretType::Primary => {
                if let Some((_, c_pos)) = self.main_caret.as_mut() {
                    c_pos.position.update(position);
                }
            }
            CaretType::Mark => {
                if let Some((_, c_pos)) = self.mark.as_mut() {
                    c_pos.position.update(position);
                }
            }
        }
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
        // update caret
        if let Some((c, i)) = self.main_caret.as_mut() {
            if !update_environment && !i.in_animation() {
                //
            } else if let Some(instance) = self.instances.get_mut(&(*c).into()) {
                update_instance(
                    instance,
                    i,
                    center,
                    position,
                    rotation,
                    calc_rotation(
                        caret_char(c.caret_type),
                        text_edit_config,
                        glyph_vertex_buffer,
                    ),
                );
            }
        }
        if let Some((c, i)) = self.mark.as_mut() {
            if !update_environment && !i.in_animation() {
                //
            } else if let Some(instance) = self.instances.get_mut(&(*c).into()) {
                update_instance(
                    instance,
                    i,
                    center,
                    position,
                    rotation,
                    calc_rotation(
                        caret_char(c.caret_type),
                        text_edit_config,
                        glyph_vertex_buffer,
                    ),
                );
            }
        }

        // update removed carets
        self.removed_carets.retain(|c, i| {
            let in_animation = i.in_animation();
            // こいつは消えゆく運命の Caret なので position_updated なんて考慮せずに in_animation だけ見る
            if !in_animation {
                self.instances.remove_from_dustbox(&(*c).into());
            }
            in_animation
        });
        for (c, i) in self.removed_carets.iter_mut() {
            if let Some(instance) = self.instances.get_mut_from_dustbox(&(*c).into()) {
                update_instance(
                    instance,
                    i,
                    center,
                    position,
                    rotation,
                    calc_rotation(
                        caret_char(c.caret_type),
                        text_edit_config,
                        glyph_vertex_buffer,
                    ),
                );
            }
        }
    }
}

#[inline]
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
    // set scale
    instance.scale = view_char_state.scale.current();
    // set gain
    instance.gain = view_char_state.motion_gain.current()[0];

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
