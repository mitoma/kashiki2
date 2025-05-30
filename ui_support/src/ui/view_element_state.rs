use std::{collections::BTreeMap, ops::Range};

use cgmath::{Matrix4, Quaternion, Rotation3, num_traits::ToPrimitive};
use instant::Duration;
use rand::Rng;
use text_buffer::{
    buffer::{BufferChar, CellPosition},
    caret::{Caret, CaretType},
};
use wgpu::Device;

use font_rasterizer::{
    char_width_calcurator::{CharWidth, CharWidthCalculator},
    color_theme::{ColorTheme, ThemedColor},
    glyph_vertex_buffer::Direction,
    motion::MotionFlags,
    time::now_millis,
    vector_instances::InstanceAttributes,
};

use crate::{
    easing_value::EasingPointN,
    layout_engine::ModelAttributes,
    text_instances::{BorderFragment, BorderInstances, BorderType, CaretInstances, TextInstances},
    ui_context::{GpuEasingConfig, RemoveCharMode, TextContext},
};

use super::caret_char;

#[derive(Default)]
pub(crate) struct ViewElementStateUpdateRequest {
    pub(crate) base_color: Option<ThemedColor>,
    pub(crate) position: Option<[f32; 3]>,
    pub(crate) color: Option<[f32; 3]>,
    pub(crate) scale: Option<[f32; 2]>,
    pub(crate) motion_gain: Option<[f32; 1]>,
}

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

// 文字の 3 次元上の位置と画面上のインスタンスを管理する構造体
#[derive(Default)]
pub(crate) struct CharStates {
    chars: BTreeMap<BufferChar, ViewElementState>,
    removed_chars: BTreeMap<BufferChar, ViewElementState>,
    pub(crate) instances: TextInstances,
}

impl CharStates {
    fn get_mut_char_and_instances(
        &mut self,
        c: &BufferChar,
    ) -> Option<(&mut ViewElementState, &mut InstanceAttributes)> {
        self.chars.get_mut(c).and_then(|state| {
            self.instances
                .get_mut(&(*c).into())
                .map(|instance| (state, instance))
        })
    }

    pub(crate) fn add_char(
        &mut self,
        c: BufferChar,
        position: [f32; 3],
        color: [f32; 3],
        counter: u32,
        text_context: &TextContext,
        device: &Device,
    ) {
        let mut easing_color = EasingPointN::new(color);
        easing_color.update_duration_and_easing_func(
            text_context.char_easings.color_easing.duration,
            text_context.char_easings.color_easing.easing_func,
        );
        let mut easing_position = EasingPointN::new(position);
        easing_position.update_duration_and_easing_func(
            text_context.char_easings.position_easing.duration,
            text_context.char_easings.position_easing.easing_func,
        );
        let mut easing_scale = EasingPointN::new(text_context.instance_scale());
        easing_scale.update_duration_and_easing_func(
            text_context.char_easings.scale_easing.duration,
            text_context.char_easings.scale_easing.easing_func,
        );
        let mut easing_motion_gain = EasingPointN::new([text_context.char_easings.add_char.gain]);
        easing_motion_gain.update_duration_and_easing_func(
            text_context.char_easings.motion_gain_easing.duration,
            text_context.char_easings.motion_gain_easing.easing_func,
        );

        let state = ViewElementState {
            position: easing_position,
            in_selection: false,
            base_color: ThemedColor::Text,
            color: easing_color,
            scale: easing_scale,
            motion_gain: easing_motion_gain,
        };
        self.chars.insert(c, state);
        let instance = InstanceAttributes {
            color,
            start_time: now_millis() + counter,
            motion: text_context.char_easings.add_char.motion,
            duration: text_context.char_easings.add_char.duration,
            ..InstanceAttributes::default()
        };
        self.instances.add(c.into(), instance, device);
    }

    pub(crate) fn move_char(
        &mut self,
        from: BufferChar,
        to: BufferChar,
        counter: u32,
        text_context: &TextContext,
        device: &Device,
    ) {
        if let Some(mut position) = self.chars.remove(&from) {
            position
                .motion_gain
                .update([text_context.char_easings.move_char.gain]);
            self.chars.insert(to, position);
        }
        if let Some(mut instance) = self.instances.remove(&from.into()) {
            instance.motion = text_context.char_easings.move_char.motion;
            if instance.start_time + instance.duration.as_millis().to_u32().unwrap() < now_millis()
            {
                instance.start_time = now_millis() + counter * 10;
            }
            instance.duration = text_context.char_easings.move_char.duration;
            self.instances.add(to.into(), instance, device);
        }
    }

    pub(crate) fn update_states(
        &mut self,
        range: &Range<CellPosition>,
        update_request: &ViewElementStateUpdateRequest,
        text_context: &TextContext,
    ) {
        let chars: Vec<BufferChar> = self
            .chars
            .keys()
            .filter(|c| range.contains(&c.position))
            .cloned()
            .collect();
        for c in chars.iter() {
            self.update_state(c, update_request, text_context);
        }
    }

    pub(crate) fn update_state(
        &mut self,
        c: &BufferChar,
        update_request: &ViewElementStateUpdateRequest,
        text_context: &TextContext,
    ) {
        if let Some(c_pos) = self.chars.get_mut(c) {
            if let Some(base_color) = update_request.base_color {
                c_pos.base_color = base_color;
                c_pos
                    .color
                    .update(base_color.get_color(&text_context.color_theme));
            }
            if let Some(position) = update_request.position {
                c_pos.position.update(position);
            }
            if let Some(color) = update_request.color {
                c_pos.color.update(color);
            }
            if let Some(scale) = update_request.scale {
                c_pos.scale.update(scale);
            }
            if let Some(motion_gain) = update_request.motion_gain {
                c_pos.motion_gain.update(motion_gain);
            }
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
        model_attribuetes: &ModelAttributes,
        char_width_calcurator: &CharWidthCalculator,
        text_context: &TextContext,
    ) {
        // update chars
        for (c, i) in self.chars.iter_mut() {
            if !update_environment && !i.in_animation() {
                continue;
            }
            if let Some(instance) = self.instances.get_mut(&(*c).into()) {
                let char_rotation = calc_rotation(c.c, text_context, char_width_calcurator);
                update_instance(instance, i, model_attribuetes, char_rotation);
            }
        }

        // update removed chars
        self.clean_dustbox();
        for (c, i) in self.removed_chars.iter_mut() {
            if !update_environment && !i.in_animation() {
                continue;
            }
            if let Some(instance) = self.instances.get_mut_from_dustbox(&(*c).into()) {
                let char_rotation = calc_rotation(c.c, text_context, char_width_calcurator);
                update_instance(instance, i, model_attribuetes, char_rotation);
            }
        }
    }

    // BufferChar をゴミ箱に移動する(削除モーションに入る)
    // remove_char_mode が Immediate の場合は即座に削除する
    pub(crate) fn char_to_dustbox(
        &mut self,
        c: BufferChar,
        counter: u32,
        text_context: &TextContext,
    ) {
        if text_context.char_easings.remove_char_mode == RemoveCharMode::Immediate {
            self.chars.remove(&c);
            self.instances.remove(&c.into());
            return;
        }

        if let Some(mut state) = self.chars.remove(&c) {
            // アニメーション状態に強制的に有効にするために gain を 0 にしている。
            // 本当はアニメーションが終わったらゴミ箱から消すという仕様が適切ではないのかもしれない
            state.motion_gain.update([0.0]);
            state
                .motion_gain
                .update([text_context.char_easings.remove_char.gain]);
            self.removed_chars.insert(c, state);
        }
        if let Some(instance) = self.instances.get_mut(&c.into()) {
            if instance.start_time + instance.duration.as_millis().to_u32().unwrap() < now_millis()
            {
                instance.start_time = now_millis() + counter * 10;
            }
            instance.motion = text_context.char_easings.remove_char.motion;
            instance.duration = text_context.char_easings.remove_char.duration;
        };
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

    pub(crate) fn set_motion_and_color(&mut self, text_context: &TextContext) {
        if text_context.psychedelic {
            let mut rng = rand::rng();
            for (c, i) in self.chars.iter_mut() {
                if let Some(instance) = self.instances.get_mut(&(*c).into()) {
                    instance.motion = MotionFlags::random_motion();
                    instance.start_time = now_millis();
                    instance.duration = Duration::from_millis(rng.random_range(300..3000));
                    i.motion_gain.update([rng.random_range(0.1..1.0)]);
                    i.base_color = match rng.random_range(0..8) {
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
                        .update(i.base_color.get_color(&text_context.color_theme));
                }
            }
        } else {
            for (_, i) in self.chars.iter_mut() {
                i.motion_gain.update([0.0]);
                i.base_color = ThemedColor::Text;
                i.color
                    .update(i.base_color.get_color(&text_context.color_theme));
            }
        }
    }

    pub(crate) fn select_char(&mut self, c: BufferChar, text_context: &TextContext) {
        if let Some((state, instance)) = self.get_mut_char_and_instances(&c) {
            state.in_selection = true;
            state.color.update(
                state
                    .base_color
                    .get_selection_color(&text_context.color_theme),
            );
            Self::apply_gpu_easing_config(&text_context.char_easings.select_char, state, instance);
        }
    }

    pub(crate) fn unselect_char(&mut self, c: BufferChar, text_context: &TextContext) {
        if let Some((state, instance)) = self.get_mut_char_and_instances(&c) {
            state.in_selection = false;
            state
                .color
                .update(state.base_color.get_color(&text_context.color_theme));
            Self::apply_gpu_easing_config(
                &text_context.char_easings.unselect_char,
                state,
                instance,
            );
        }
    }

    pub(crate) fn notify_char(&mut self, c: BufferChar, text_context: &TextContext) {
        if let Some((state, instance)) = self.get_mut_char_and_instances(&c) {
            Self::apply_gpu_easing_config(&text_context.char_easings.notify_char, state, instance);
        }
    }

    fn apply_gpu_easing_config(
        gpu_easing_config: &GpuEasingConfig,
        state: &mut ViewElementState,
        instance: &mut InstanceAttributes,
    ) {
        state.motion_gain.update([gpu_easing_config.gain]);
        instance.motion = gpu_easing_config.motion;
        instance.duration = gpu_easing_config.duration;
        instance.start_time = now_millis();
    }

    pub(crate) fn get_nearest_char(
        &self,
        x_ratio: f32,
        y_ratio: f32,
        view_projection_matrix: &Matrix4<f32>,
        model_attribuetes: &ModelAttributes,
    ) -> Option<BufferChar> {
        let mut distance_map: BTreeMap<BufferChar, f32> = BTreeMap::new();

        for (idx, state) in self.chars.iter() {
            let [x, y, z] = state.position.current();
            let pos = cgmath::Matrix4::from(model_attribuetes.rotation)
                * cgmath::Matrix4::from_translation(cgmath::Vector3 { x, y, z }).w;
            let new_position = cgmath::Vector3 {
                x: pos.x - model_attribuetes.center.x + model_attribuetes.position.x,
                y: pos.y - model_attribuetes.center.y + model_attribuetes.position.y,
                z: pos.z + model_attribuetes.position.z,
            };
            let new_position = cgmath::Matrix4::from_translation(new_position);
            let calced_model_position = view_projection_matrix * new_position;
            let nw = calced_model_position.w;
            let nw_x = nw.x / nw.w;
            let nw_y = nw.y / nw.w;

            let distance = (x_ratio - nw_x).abs().powf(2.0) + (y_ratio - nw_y).abs().powf(2.0);
            distance_map.insert(*idx, distance);
        }

        let min_distance = distance_map
            .iter()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap());
        min_distance.map(|(c, _)| *c)
    }
}

#[derive(Default)]
pub(crate) struct CaretStates {
    main_caret: Option<(Caret, ViewElementState)>,
    mark: Option<(Caret, ViewElementState)>,
    removed_carets: BTreeMap<Caret, ViewElementState>,
    pub(crate) instances: CaretInstances,
}

impl CaretStates {
    pub(crate) fn main_caret_logical_position(&self) -> Option<[usize; 2]> {
        self.main_caret
            .as_ref()
            .map(|(c, _)| [c.position.row, c.position.col])
    }

    pub(crate) fn main_caret_position(&self) -> Option<[f32; 3]> {
        self.main_caret.as_ref().map(|(_, s)| s.position.last())
    }

    pub(crate) fn add_caret(
        &mut self,
        c: Caret,
        color: [f32; 3],
        text_context: &TextContext,
        device: &Device,
    ) {
        let position = self.main_caret_position().unwrap_or([0.0, 0.0, 0.0]);
        let mut easing_position = EasingPointN::new(position);
        easing_position.update_duration_and_easing_func(
            text_context.char_easings.position_easing.duration,
            text_context.char_easings.position_easing.easing_func,
        );

        let mut easing_color = EasingPointN::new(text_context.color_theme.background().get_color());
        easing_color.update_duration_and_easing_func(
            text_context.char_easings.color_easing.duration,
            text_context.char_easings.color_easing.easing_func,
        );
        easing_color.update(color);

        let mut easing_scale = EasingPointN::new(text_context.instance_scale());
        easing_scale.update_duration_and_easing_func(
            text_context.char_easings.scale_easing.duration,
            text_context.char_easings.scale_easing.easing_func,
        );
        let mut easing_motion_gain = EasingPointN::new([text_context.char_easings.add_char.gain]);
        easing_motion_gain.update_duration_and_easing_func(
            text_context.char_easings.motion_gain_easing.duration,
            text_context.char_easings.motion_gain_easing.easing_func,
        );
        let state = ViewElementState {
            position: easing_position,
            in_selection: false,
            base_color: ThemedColor::TextEmphasized,
            color: easing_color,
            scale: easing_scale,
            motion_gain: easing_motion_gain,
        };
        if c.caret_type == CaretType::Primary {
            self.main_caret.replace((c, state));
        } else {
            self.mark.replace((c, state));
        }

        let caret_instance = InstanceAttributes {
            color,
            start_time: now_millis(),
            motion: text_context.char_easings.add_caret.motion,
            duration: text_context.char_easings.add_caret.duration,
            gain: text_context.char_easings.add_caret.gain,
            ..InstanceAttributes::default()
        };
        self.instances.add(c.into(), caret_instance, device);
    }

    pub(crate) fn move_caret(
        &mut self,
        from: Caret,
        to: Caret,
        text_context: &TextContext,
        device: &Device,
    ) {
        match from.caret_type {
            CaretType::Primary => self.main_caret = Some((to, self.main_caret.take().unwrap().1)),
            CaretType::Mark => self.mark = Some((to, self.mark.take().unwrap().1)),
        }

        if let Some(mut instance) = self.instances.remove(&from.into()) {
            instance.start_time = now_millis();
            instance.motion = text_context.char_easings.move_caret.motion;
            instance.duration = text_context.char_easings.move_caret.duration;
            instance.gain = text_context.char_easings.move_caret.gain;
            self.instances.add(to.into(), instance, device);
        }
    }

    // BufferChar をゴミ箱に移動する(削除モーションに入る)
    pub(crate) fn caret_to_dustbox(&mut self, c: Caret, text_context: &TextContext) {
        match c.caret_type {
            CaretType::Primary => {
                if let Some((_, mut state)) = self.main_caret.take() {
                    state
                        .color
                        .update(text_context.color_theme.background().get_color());
                    self.removed_carets.insert(c, state);
                }
            }
            CaretType::Mark => {
                if let Some((_, mut state)) = self.mark.take() {
                    state
                        .color
                        .update(text_context.color_theme.background().get_color());
                    self.removed_carets.insert(c, state);
                }
            }
        }
        if let Some(attr) = self.instances.get_mut(&c.into()) {
            attr.start_time = now_millis();
            attr.motion = text_context.char_easings.remove_caret.motion;
            attr.duration = text_context.char_easings.remove_caret.duration;
            attr.gain = text_context.char_easings.remove_caret.gain;
        }
        self.instances.pre_remove(&c.into());
    }

    pub(crate) fn update_state_position_and_scale(
        &mut self,
        caret_type: CaretType,
        position: [f32; 3],
        config: &TextContext,
    ) {
        #[inline]
        fn update_view_element_state(
            state: &mut ViewElementState,
            position: [f32; 3],
            config: &TextContext,
        ) {
            state.position.update(position);
            state.scale.update(config.instance_scale());
            state
                .color
                .update(state.base_color.get_color(&config.color_theme));
        }

        match caret_type {
            CaretType::Primary => {
                if let Some((_, state)) = self.main_caret.as_mut() {
                    update_view_element_state(state, position, config);
                }
            }
            CaretType::Mark => {
                if let Some((_, state)) = self.mark.as_mut() {
                    update_view_element_state(state, position, config);
                }
            }
        }
    }

    pub(crate) fn update_instances(
        &mut self,
        update_environment: bool,
        model_attribuetes: &ModelAttributes,
        char_width_calcurator: &CharWidthCalculator,
        text_context: &TextContext,
    ) {
        // update caret
        if let Some((c, i)) = self.main_caret.as_mut() {
            if !update_environment && !i.in_animation() {
                //
            } else if let Some(instance) = self.instances.get_mut(&(*c).into()) {
                update_instance(
                    instance,
                    i,
                    model_attribuetes,
                    calc_rotation(
                        caret_char(c.caret_type),
                        text_context,
                        char_width_calcurator,
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
                    model_attribuetes,
                    calc_rotation(
                        caret_char(c.caret_type),
                        text_context,
                        char_width_calcurator,
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
                    model_attribuetes,
                    calc_rotation(
                        caret_char(c.caret_type),
                        text_context,
                        char_width_calcurator,
                    ),
                );
            }
        }
    }
}

#[inline]
fn update_instance(
    instance: &mut InstanceAttributes,
    view_char_state: &mut ViewElementState,
    model_attribuetes: &ModelAttributes,
    char_rotation: Option<Quaternion<f32>>,
) {
    // set color
    instance.color = view_char_state.color.current();
    // set scale
    instance.world_scale = model_attribuetes.world_scale;
    // グリフの回転が入る場合は scale を入れ替える必要がある
    instance.instance_scale = if char_rotation.is_some() {
        let [l, r] = view_char_state.scale.current();
        [r, l]
    } else {
        view_char_state.scale.current()
    };
    // set gain
    instance.gain = view_char_state.motion_gain.current()[0];

    // set position
    let [x, y, z] = view_char_state.position.current();
    // モデル全体を回転させた後にモデルとしての中心を真ん中に移動する
    let pos = (cgmath::Matrix4::from(model_attribuetes.rotation)
        * cgmath::Matrix4::from_translation(cgmath::Vector3 {
            x: x - model_attribuetes.center.x,
            y: y - model_attribuetes.center.y,
            z,
        }))
    .w;
    // そのあと、World に対しての位置を考慮して移動する
    let new_position = cgmath::Vector3 {
        x: pos.x + model_attribuetes.position.x,
        y: pos.y + model_attribuetes.position.y,
        z: pos.z + model_attribuetes.position.z,
    };
    instance.position = new_position;

    // set rotation
    // 縦書きの場合は char_rotation が必要なのでここで回転する
    // TODO: rotation を変更したときに vertex shader での motion も考慮する必要がある
    instance.rotation = match char_rotation {
        Some(r) => model_attribuetes.rotation * r,
        None => model_attribuetes.rotation,
    }
}

#[inline]
fn calc_rotation(
    c: char,
    text_context: &TextContext,
    char_width_calcurator: &CharWidthCalculator,
) -> Option<Quaternion<f32>> {
    match text_context.direction {
        Direction::Horizontal => None,
        Direction::Vertical => {
            let width = char_width_calcurator.get_width(c);
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

const BORDER_TOP: BorderFragment = BorderFragment {
    border_type: BorderType::Horizontal,
    position: CellPosition { row: 0, col: 1 },
};

const BORDER_BOTTOM: BorderFragment = BorderFragment {
    border_type: BorderType::Horizontal,
    position: CellPosition { row: 2, col: 1 },
};

const BORDER_LEFT: BorderFragment = BorderFragment {
    border_type: BorderType::Vertical,
    position: CellPosition { row: 1, col: 0 },
};

const BORDER_RIGHT: BorderFragment = BorderFragment {
    border_type: BorderType::Vertical,
    position: CellPosition { row: 1, col: 2 },
};

const BORDER_TOP_LEFT: BorderFragment = BorderFragment {
    border_type: BorderType::TopLeft,
    position: CellPosition { row: 0, col: 0 },
};

const BORDER_TOP_RIGHT: BorderFragment = BorderFragment {
    border_type: BorderType::TopRight,
    position: CellPosition { row: 0, col: 2 },
};

const BORDER_BOTTOM_LEFT: BorderFragment = BorderFragment {
    border_type: BorderType::BottomLeft,
    position: CellPosition { row: 2, col: 0 },
};

const BORDER_BOTTOM_RIGHT: BorderFragment = BorderFragment {
    border_type: BorderType::BottomRight,
    position: CellPosition { row: 2, col: 2 },
};

/// 罫線の 3 次元上の位置と画面上のインスタンスを管理する構造体
pub(crate) struct BorderStates {
    initialized: bool,
    elements: BTreeMap<BorderFragment, ViewElementState>,
    pub(crate) instances: BorderInstances,
}

impl Default for BorderStates {
    fn default() -> Self {
        Self::new()
    }
}

impl BorderStates {
    pub(crate) fn new() -> Self {
        Self {
            initialized: false,
            elements: BTreeMap::default(),
            instances: BorderInstances::default(),
        }
    }

    pub(crate) fn init(&mut self, text_context: &TextContext, device: &Device) {
        if self.initialized {
            return;
        }
        self.add_fragment(BORDER_TOP, text_context, device);
        self.add_fragment(BORDER_BOTTOM, text_context, device);
        self.add_fragment(BORDER_LEFT, text_context, device);
        self.add_fragment(BORDER_RIGHT, text_context, device);
        self.add_fragment(BORDER_TOP_LEFT, text_context, device);
        self.add_fragment(BORDER_TOP_RIGHT, text_context, device);
        self.add_fragment(BORDER_BOTTOM_LEFT, text_context, device);
        self.add_fragment(BORDER_BOTTOM_RIGHT, text_context, device);
        self.initialized = true;
    }

    pub(crate) fn update_state(
        &mut self,
        position: [f32; 3],
        bound: [f32; 2],
        config: &TextContext,
    ) {
        // TODO: scale の扱いがおかしいので後で修正する。
        // 現状、グリフの拡大縮小をしたときに罫線の位置とサイズの追従が適切に行えていない。
        let [sx, sy] = config.instance_scale();
        if let Some(state) = self.elements.get_mut(&BORDER_TOP) {
            state.position.update([
                position[0] + (sx * bound[0] / 2.0),
                position[1] + 1.0,
                position[2],
            ]);
            state.scale.update([sx * bound[0] + 1.0, sy]);
        }
        if let Some(state) = self.elements.get_mut(&BORDER_BOTTOM) {
            state.position.update([
                position[0] + (sx * bound[0] / 2.0),
                position[1] - bound[1] - 1.0,
                position[2],
            ]);
            state.scale.update([sx * bound[0] + 1.0, sy]);
        }
        if let Some(state) = self.elements.get_mut(&BORDER_LEFT) {
            state.position.update([
                position[0] - 1.0,
                position[1] - (sy * bound[1] / 2.0),
                position[2],
            ]);
            state.scale.update([sx, sy * bound[1] + 1.0]);
        }
        if let Some(state) = self.elements.get_mut(&BORDER_RIGHT) {
            state.position.update([
                position[0] + bound[0] + 1.0,
                position[1] - (sy * bound[1] / 2.0),
                position[2],
            ]);
            state.scale.update([sx, sy * bound[1] + 1.0]);
        }

        if let Some(state) = self.elements.get_mut(&BORDER_TOP_LEFT) {
            state
                .position
                .update([position[0] - 1.0, position[1] + 1.0, position[2]]);
            state.scale.update([sx, sy]);
        }
        if let Some(state) = self.elements.get_mut(&BORDER_TOP_RIGHT) {
            state
                .position
                .update([position[0] + bound[0] + 1.0, position[1] + 1.0, position[2]]);
            state.scale.update([sx, sy]);
        }
        if let Some(state) = self.elements.get_mut(&BORDER_BOTTOM_LEFT) {
            state
                .position
                .update([position[0] - 1.0, position[1] - bound[1] - 1.0, position[2]]);
            state.scale.update([sx, sy]);
        }
        if let Some(state) = self.elements.get_mut(&BORDER_BOTTOM_RIGHT) {
            state.position.update([
                position[0] + bound[0] + 1.0,
                position[1] - bound[1] - 1.0,
                position[2],
            ]);
            state.scale.update([sx, sy]);
        }
    }

    pub(crate) fn update_instances(
        &mut self,
        update_environment: bool,
        model_attribuetes: &ModelAttributes,
    ) {
        if !self.initialized {
            return;
        }

        // update fragments
        self.elements.iter_mut().for_each(|(fragment, state)| {
            if !update_environment && !state.in_animation() {
                return;
            }
            if let Some(instance) = self.instances.get_mut(fragment) {
                update_instance(instance, state, model_attribuetes, None);
            }
        });
    }

    fn add_fragment(
        &mut self,
        fragment: BorderFragment,
        text_context: &TextContext,
        device: &Device,
    ) {
        let base_color = ThemedColor::TextEmphasized;
        let position = [
            fragment.position.col as f32,
            fragment.position.row as f32,
            0.0,
        ];
        let state = ViewElementState {
            base_color,
            in_selection: false,
            position: position.into(),
            color: base_color.get_color(&text_context.color_theme).into(),
            scale: [1.0, 1.0].into(),
            motion_gain: [1.0].into(),
        };
        self.elements.insert(fragment.clone(), state);
        self.instances
            .add(fragment, InstanceAttributes::default(), device);
    }
}
