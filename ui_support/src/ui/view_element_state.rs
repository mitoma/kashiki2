use std::{collections::BTreeMap, collections::BTreeSet, ops::Range};

use glam::{Mat4, Quat, Vec3};
use rand::RngExt;
use text_buffer::{
    buffer::{BufferChar, CellPosition},
    caret::{Caret, CaretType},
};
use web_time::Duration;
use wgpu::Device;

use font_rasterizer::{
    char_width_calcurator::{CharWidth, CharWidthCalculator},
    color_theme::{ColorTheme, ThemedColor},
    glyph_vertex_buffer::Direction,
    motion::MotionFlags,
    time::now_millis,
    vector_instances::{InstanceAttributes, InstanceKey, VectorInstances},
};

use crate::{
    easing_value::EasingPointN,
    layout_engine::{MODEL_SURFACE_SVG_KEY, ModelAttributes, ModelBorder, ModelSurfaceStyle},
    retiring::{Retiring, retire_deadline},
    text_instances::{BorderFragment, BorderInstances, BorderType, CaretInstances, TextInstances},
    ui_context::{CharEasings, GpuEasingConfig, RemoveCharMode, TextContext, UiContext},
};

use super::caret_char;

pub(crate) fn update_model_surface_instances(
    instances: &mut Option<VectorInstances<String>>,
    style: ModelSurfaceStyle,
    geometry_key: &str,
    mut attributes: InstanceAttributes,
    context: &UiContext,
) {
    let (color, alpha) = match style {
        ModelSurfaceStyle::None => {
            *instances = None;
            return;
        }
        ModelSurfaceStyle::ThemeBackground { alpha } => {
            (context.color_theme().background().get_color(), alpha)
        }
        ModelSurfaceStyle::ThemeBackgroundHighlight { alpha } => (
            context.color_theme().background_highlights().get_color(),
            alpha,
        ),
        ModelSurfaceStyle::Color([r, g, b, a]) => ([r, g, b], a),
    };
    if instances
        .as_ref()
        .is_some_and(|instances| instances.key != geometry_key)
    {
        *instances = None;
    }
    let instances = instances
        .get_or_insert_with(|| VectorInstances::new(geometry_key.to_owned(), context.device()));
    attributes.color = color;
    attributes.alpha = alpha.clamp(0.0, 1.0);
    instances.insert(InstanceKey::Monotonic(0), attributes);
    instances.update_buffer(context.device(), context.queue());
}

/// 面は bound の外側にある罫線(線幅 1/9 セル)の内縁まで広げる
pub(crate) const SURFACE_OUTSET: f32 = BORDER_OUTSET - 0.5 / 9.0;
/// 角丸罫線の内縁と同じ半径
const SURFACE_CORNER_RADIUS: f32 = 4.0 / 9.0;

/// 罫線が Square のときだけ四角い面にし、None と Rounded では角丸罫線と同じ輪郭にする
pub(crate) fn surface_corner_radius(border: ModelBorder) -> f32 {
    if border == ModelBorder::Square {
        0.0
    } else {
        SURFACE_CORNER_RADIUS
    }
}

/// 面の geometry を管理する。角丸の面は最終サイズごとに SVG を登録する
#[derive(Default)]
pub(crate) struct SurfaceGeometry {
    registered_key: Option<String>,
}

impl SurfaceGeometry {
    /// geometry key と instance_scale を返す
    pub(crate) fn resolve(
        &mut self,
        last_size: [f32; 2],
        current_size: [f32; 2],
        radius: f32,
        context: &UiContext,
    ) -> (String, [f32; 2]) {
        if radius <= 0.0 {
            return (MODEL_SURFACE_SVG_KEY.to_owned(), current_size);
        }
        let [width, height] = last_size;
        let key = format!(
            "{}_rounded_{:08x}_{:08x}_{:08x}",
            MODEL_SURFACE_SVG_KEY,
            width.to_bits(),
            height.to_bits(),
            radius.to_bits(),
        );
        if self.registered_key.as_deref() != Some(key.as_str()) {
            context.register_svg(key.clone(), rounded_surface_svg(width, height, radius));
            self.registered_key = Some(key.clone());
        }
        // SVG は長辺で正規化されるため、長辺を基準に現在のサイズへ伸縮する
        let size_scale = width.max(height);
        let [current_width, current_height] = current_size;
        (
            key,
            [
                size_scale * current_width / width,
                size_scale * current_height / height,
            ],
        )
    }
}

fn rounded_surface_svg(width: f32, height: f32, radius: f32) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\"><path d=\"M {radius} 0 H {} Q {width} 0 {width} {radius} V {} Q {width} {height} {} {height} H {radius} Q 0 {height} 0 {} V {radius} Q 0 0 {radius} 0 Z\" /></svg>",
        width - radius,
        height - radius,
        width - radius,
        height - radius,
    )
}

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
    preedit_chars: BTreeMap<BufferChar, ViewElementState>,
    removed_chars: Retiring<(BufferChar, ViewElementState)>,
    pub(crate) instances: TextInstances,
}

impl CharStates {
    fn get_mut_char_and_instances(
        &mut self,
        c: &BufferChar,
    ) -> Option<(&mut ViewElementState, &mut InstanceAttributes)> {
        self.chars
            .get_mut(c)
            .zip(self.instances.get_mut(&(*c).into()))
    }

    pub(crate) fn add_preedit_char(
        &mut self,
        c: BufferChar,
        position: [f32; 3],
        color: [f32; 3],
        counter: u32,
        text_context: &TextContext,
        device: &Device,
    ) {
        let (state, instance) =
            self.crate_state_and_attribute(position, color, counter, text_context);

        self.preedit_chars.insert(c, state);
        self.instances.add_preedit(c.into(), instance, device);
    }

    pub(crate) fn sync_preedit_chars(
        &mut self,
        next_chars: &[BufferChar],
        has_selection: bool,
        initial_position: [f32; 3],
        text_context: &TextContext,
        device: &Device,
    ) {
        let prev_set: BTreeSet<BufferChar> = self.preedit_chars.keys().copied().collect();
        let next_set: BTreeSet<BufferChar> = next_chars.iter().copied().collect();

        // 確定された preedit は sync_editor_events で文字に昇格済みなので、ここに残るのは取り消された文字
        for (counter, c) in prev_set.difference(&next_set).enumerate() {
            let Some(mut state) = self.preedit_chars.remove(c) else {
                continue;
            };
            let deadline = start_char_retirement(
                &mut state,
                self.instances.get_mut_preedit(&(*c).into()),
                counter as u32,
                text_context,
            );
            let id = self.removed_chars.retire((*c, state), deadline);
            self.instances.retire_preedit(&(*c).into(), id);
        }

        for c in next_chars.iter().copied() {
            let base_color = if has_selection && (c.c == '[' || c.c == ']') {
                ThemedColor::TextComment
            } else {
                ThemedColor::Text
            };

            if !self.preedit_chars.contains_key(&c) {
                self.add_preedit_char(
                    c,
                    initial_position,
                    base_color.get_color(&text_context.color_theme),
                    0,
                    text_context,
                    device,
                );
            }

            self.update_preedit_state(
                &c,
                &ViewElementStateUpdateRequest {
                    base_color: Some(base_color),
                    ..Default::default()
                },
                text_context,
            );
        }
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
        let (state, instance) =
            self.crate_state_and_attribute(position, color, counter, text_context);

        self.chars.insert(c, state);
        self.instances.add(c.into(), instance, device);
    }

    pub(crate) fn promote_preedit_to_char(&mut self, to: BufferChar, device: &Device) -> bool {
        // まずは完全一致（同一セル・同一文字）を優先する。
        let from = if self.preedit_chars.contains_key(&to) {
            Some(to)
        } else {
            self.preedit_chars
                .keys()
                .filter(|candidate| candidate.c == to.c)
                .min_by_key(|candidate| {
                    let row_distance = candidate.position.row.abs_diff(to.position.row);
                    let col_distance = candidate.position.col.abs_diff(to.position.col);
                    row_distance * 1_000 + col_distance
                })
                .copied()
        };

        let Some(from) = from else {
            return false;
        };

        let Some(mut state) = self.preedit_chars.remove(&from) else {
            return false;
        };
        state.base_color = ThemedColor::Text;
        state.in_selection = false;
        self.chars.insert(to, state);

        if let Some(instance) = self.instances.remove_preedit(&from.into()) {
            self.instances.add(to.into(), instance, device);
        }

        true
    }

    #[inline]
    fn crate_state_and_attribute(
        &mut self,
        position: [f32; 3],
        color: [f32; 3],
        counter: u32,
        text_context: &TextContext,
    ) -> (ViewElementState, InstanceAttributes) {
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
        let instance = InstanceAttributes {
            color,
            start_time: now_millis() + counter,
            motion: text_context.char_easings.add_char.motion,
            duration: text_context.char_easings.add_char.duration,
            ..InstanceAttributes::default()
        };
        (state, instance)
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
            if instance.start_time + (instance.duration.as_millis() as u32) < now_millis() {
                instance.start_time = now_millis() + counter;
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
        if self.chars.contains_key(c) {
            self.update_char_state(c, update_request, text_context);
        } else if self.preedit_chars.contains_key(c) {
            self.update_preedit_state(c, update_request, text_context);
        }
    }

    fn update_char_state(
        &mut self,
        c: &BufferChar,
        update_request: &ViewElementStateUpdateRequest,
        text_context: &TextContext,
    ) {
        if let Some(c_pos) = self.chars.get_mut(c) {
            Self::apply_update_request(c_pos, update_request, text_context);
        }
    }

    fn update_preedit_state(
        &mut self,
        c: &BufferChar,
        update_request: &ViewElementStateUpdateRequest,
        text_context: &TextContext,
    ) {
        if let Some(c_pos) = self.preedit_chars.get_mut(c) {
            Self::apply_update_request(c_pos, update_request, text_context);
        }
    }

    fn apply_update_request(
        c_pos: &mut ViewElementState,
        update_request: &ViewElementStateUpdateRequest,
        text_context: &TextContext,
    ) {
        update_easings(c_pos, &text_context.char_easings);
        if let Some(base_color) = update_request.base_color {
            c_pos.base_color = base_color;
            let color = if c_pos.in_selection {
                c_pos
                    .base_color
                    .get_selection_color(&text_context.color_theme)
            } else {
                c_pos.base_color.get_color(&text_context.color_theme)
            };

            c_pos.color.update(color);
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

    pub(crate) fn update_char_theme(&mut self, color_theme: &ColorTheme) {
        self.chars.iter_mut().for_each(|(_, i)| {
            let color = if i.in_selection {
                i.base_color.get_selection_color(color_theme)
            } else {
                i.base_color.get_color(color_theme)
            };
            i.color.update(color);
        });

        self.preedit_chars.iter_mut().for_each(|(_, i)| {
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
        self.sweep_removed_chars();
        for (id, (c, i)) in self.removed_chars.iter_mut() {
            if !update_environment && !i.in_animation() {
                continue;
            }
            if let Some(instance) = self.instances.get_mut_retired(c.c, id) {
                let char_rotation = calc_rotation(c.c, text_context, char_width_calcurator);
                update_instance(instance, i, model_attribuetes, char_rotation);
            }
        }

        // update preedit chars
        for (c, i) in self.preedit_chars.iter_mut() {
            if !update_environment && !i.in_animation() {
                continue;
            }
            if let Some(instance) = self.instances.get_mut_preedit(&(*c).into()) {
                let char_rotation = calc_rotation(c.c, text_context, char_width_calcurator);
                update_instance(instance, i, model_attribuetes, char_rotation);
            }
        }
    }

    // 文字を削除アニメーションに入れ、アニメーションが終わる時刻に破棄されるようにする。
    // remove_char_mode が Immediate の場合は次の更新で破棄される
    pub(crate) fn retire_char(&mut self, c: BufferChar, counter: u32, text_context: &TextContext) {
        let Some(mut state) = self.chars.remove(&c) else {
            self.instances.remove(&c.into());
            return;
        };
        let deadline = start_char_retirement(
            &mut state,
            self.instances.get_mut(&c.into()),
            counter,
            text_context,
        );
        let id = self.removed_chars.retire((c, state), deadline);
        self.instances.retire(&c.into(), id);
    }

    fn sweep_removed_chars(&mut self) {
        let instances = &mut self.instances;
        self.removed_chars.sweep(now_millis(), |id, (c, _)| {
            instances.remove_retired(c.c, id);
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
            for i in self.chars.values_mut() {
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
        view_projection_matrix: &Mat4,
        model_attribuetes: &ModelAttributes,
    ) -> Option<BufferChar> {
        let mut distance_map: BTreeMap<BufferChar, f32> = BTreeMap::new();

        for (idx, state) in self.chars.iter() {
            let [x, y, z] = state.position.current();
            let pos = Mat4::from_quat(model_attribuetes.rotation)
                .mul_mat4(&Mat4::from_translation(Vec3::new(x, y, z)))
                .w_axis;
            let new_position = Vec3 {
                x: pos.x - model_attribuetes.center.x + model_attribuetes.position.x,
                y: pos.y - model_attribuetes.center.y + model_attribuetes.position.y,
                z: pos.z + model_attribuetes.position.z,
            };
            let new_position = Mat4::from_translation(new_position);
            let calced_model_position = view_projection_matrix * new_position;
            let nw = calced_model_position.w_axis;
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
    removed_carets: Retiring<(Caret, ViewElementState)>,
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

    // Caret を削除アニメーションに入れ、アニメーションが終わる時刻に破棄されるようにする
    pub(crate) fn retire_caret(&mut self, c: Caret, text_context: &TextContext) {
        let taken = match c.caret_type {
            CaretType::Primary => self.main_caret.take(),
            CaretType::Mark => self.mark.take(),
        };
        let Some((_, mut state)) = taken else {
            self.instances.remove(&c.into());
            return;
        };
        let now = now_millis();
        let char_easings = &text_context.char_easings;
        update_easings(&mut state, char_easings);
        state
            .color
            .update(text_context.color_theme.background().get_color());
        if let Some(attr) = self.instances.get_mut(&c.into()) {
            attr.start_time = now;
            attr.motion = char_easings.remove_caret.motion;
            attr.duration = char_easings.remove_caret.duration;
            attr.gain = char_easings.remove_caret.gain;
        }
        let deadline = retire_deadline(
            now,
            now,
            char_easings.remove_caret.duration,
            char_easings.max_cpu_easing_duration(),
        );
        let id = self.removed_carets.retire((c, state), deadline);
        self.instances.retire(&c.into(), id);
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
            update_easings(state, &config.char_easings);
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
        let instances = &mut self.instances;
        self.removed_carets.sweep(now_millis(), |id, (c, _)| {
            instances.remove_retired(caret_char(c.caret_type), id);
        });
        for (id, (c, i)) in self.removed_carets.iter_mut() {
            let caret_char = caret_char(c.caret_type);
            if let Some(instance) = self.instances.get_mut_retired(caret_char, id) {
                update_instance(
                    instance,
                    i,
                    model_attribuetes,
                    calc_rotation(caret_char, text_context, char_width_calcurator),
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
    char_rotation: Option<Quat>,
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
    let pos =
        (Mat4::from_quat(model_attribuetes.rotation).mul_mat4(&Mat4::from_translation(Vec3 {
            x: x - model_attribuetes.center.x,
            y: y - model_attribuetes.center.y,
            z,
        })))
        .w_axis;
    // そのあと、World に対しての位置を考慮して移動する
    let new_position = Vec3 {
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
) -> Option<Quat> {
    match text_context.direction {
        Direction::Horizontal => None,
        Direction::Vertical => {
            let width = char_width_calcurator.get_width(c);
            match width {
                CharWidth::Regular => Some(Quat::from_axis_angle(Vec3::Z, -90.0f32.to_radians())),
                CharWidth::Wide => None,
                CharWidth::Proportional(width) if width < 1.0 => {
                    Some(Quat::from_axis_angle(Vec3::Z, -90.0f32.to_radians()))
                }
                CharWidth::Proportional(_) => None,
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

const BORDER_BASE_COLOR: ThemedColor = ThemedColor::TextEmphasized;

/// 罫線は bound の外側にこの距離だけ離して描画される
pub(crate) const BORDER_OUTSET: f32 = 1.0;

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
        let color = BORDER_BASE_COLOR.get_color(&config.color_theme);
        for state in self.elements.values_mut() {
            state.color.update(color);
        }

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

    // 罫線を退場させる。色を背景色へ寄せ、破棄してよい時刻を返す
    pub(crate) fn begin_retire(&mut self, text_context: &TextContext) -> u32 {
        let char_easings = &text_context.char_easings;
        let background = text_context.color_theme.background().get_color();
        for state in self.elements.values_mut() {
            update_easings(state, char_easings);
            state.color.update(background);
        }
        now_millis().saturating_add(char_easings.max_cpu_easing_duration().as_millis() as u32)
    }

    fn add_fragment(
        &mut self,
        fragment: BorderFragment,
        text_context: &TextContext,
        device: &Device,
    ) {
        let position = [
            fragment.position.col as f32,
            fragment.position.row as f32,
            0.0,
        ];
        let state = ViewElementState {
            base_color: BORDER_BASE_COLOR,
            in_selection: false,
            position: position.into(),
            color: BORDER_BASE_COLOR
                .get_color(&text_context.color_theme)
                .into(),
            scale: [1.0, 1.0].into(),
            motion_gain: [1.0].into(),
        };
        self.elements.insert(fragment.clone(), state);
        // 罫線はアニメーションしないが、カメラを無視する設定は文字と揃える
        let motion = MotionFlags::builder()
            .camera_detail(text_context.char_easings.add_char.motion.camera_detail())
            .build();
        self.instances.add(
            fragment,
            InstanceAttributes {
                motion,
                ..InstanceAttributes::default()
            },
            device,
        );
    }
}

// 文字の削除モーションを開始し、破棄してよい時刻を返す
fn start_char_retirement(
    state: &mut ViewElementState,
    instance: Option<&mut InstanceAttributes>,
    counter: u32,
    text_context: &TextContext,
) -> u32 {
    let now = now_millis();
    let char_easings = &text_context.char_easings;
    match (char_easings.remove_char_mode, instance) {
        (RemoveCharMode::Delayed, Some(instance)) => {
            if instance.start_time + (instance.duration.as_millis() as u32) < now {
                instance.start_time = now + counter * 10;
            }
            instance.motion = char_easings.remove_char.motion;
            instance.duration = char_easings.remove_char.duration;
            update_easings(state, char_easings);
            state.motion_gain.update([char_easings.remove_char.gain]);
            retire_deadline(
                now,
                instance.start_time,
                instance.duration,
                char_easings.max_cpu_easing_duration(),
            )
        }
        _ => now,
    }
}

fn update_easings(view_char_state: &mut ViewElementState, char_easings: &CharEasings) {
    view_char_state.position.update_duration_and_easing_func(
        char_easings.position_easing.duration,
        char_easings.position_easing.easing_func,
    );
    view_char_state.color.update_duration_and_easing_func(
        char_easings.color_easing.duration,
        char_easings.color_easing.easing_func,
    );
    view_char_state.scale.update_duration_and_easing_func(
        char_easings.scale_easing.duration,
        char_easings.scale_easing.easing_func,
    );
    view_char_state.motion_gain.update_duration_and_easing_func(
        char_easings.motion_gain_easing.duration,
        char_easings.motion_gain_easing.easing_func,
    );
}

#[cfg(test)]
mod border_theme_tests {
    use super::*;

    #[test]
    fn border_update_tracks_text_emphasized_color_for_all_fragments() {
        let initial_color = ThemedColor::TextEmphasized.get_color(&ColorTheme::SolarizedLight);
        let mut states = BorderStates::new();
        for fragment in [
            BORDER_TOP,
            BORDER_BOTTOM,
            BORDER_LEFT,
            BORDER_RIGHT,
            BORDER_TOP_LEFT,
            BORDER_TOP_RIGHT,
            BORDER_BOTTOM_LEFT,
            BORDER_BOTTOM_RIGHT,
        ] {
            let mut color = EasingPointN::new(initial_color);
            color.update_duration_and_easing_func(Duration::ZERO, nenobi::functions::sin_out);
            states.elements.insert(
                fragment,
                ViewElementState {
                    base_color: ThemedColor::TextEmphasized,
                    in_selection: false,
                    position: [0.0; 3].into(),
                    color,
                    scale: [1.0; 2].into(),
                    motion_gain: [1.0].into(),
                },
            );
        }
        assert_eq!(states.elements.len(), 8);
        let model_attributes = ModelAttributes {
            center: Default::default(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            world_scale: [1.0; 2],
        };

        for color_theme in [
            ColorTheme::SolarizedDark,
            ColorTheme::SolarizedBlackback,
            ColorTheme::SolarizedLight,
            ColorTheme::HighContrastLight,
            ColorTheme::HighContrastDark,
            ColorTheme::WarmLight,
            ColorTheme::WarmDark,
            ColorTheme::CoolLight,
            ColorTheme::CoolDark,
            ColorTheme::Vivid,
        ] {
            let config = TextContext {
                color_theme,
                ..Default::default()
            };
            let expected_color = ThemedColor::TextEmphasized.get_color(&color_theme);
            for _ in 0..2 {
                states.update_state([0.0; 3], [4.0, 3.0], &config);
                for state in states.elements.values_mut() {
                    let mut instance = InstanceAttributes::default();
                    update_instance(&mut instance, state, &model_attributes, None);
                    for color in [state.color.last(), instance.color] {
                        for (actual, expected) in color.into_iter().zip(expected_color) {
                            assert!(
                                (actual - expected).abs() < 1e-6,
                                "border color: {actual}, expected: {expected}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn retiring_border_fades_to_background_until_easings_finish() {
        let mut states = BorderStates::new();
        for fragment in [BORDER_TOP, BORDER_BOTTOM_RIGHT] {
            states.elements.insert(
                fragment,
                ViewElementState {
                    base_color: BORDER_BASE_COLOR,
                    in_selection: false,
                    position: [0.0; 3].into(),
                    color: [1.0; 3].into(),
                    scale: [1.0; 2].into(),
                    motion_gain: [1.0].into(),
                },
            );
        }
        let config = TextContext::default();
        let before = now_millis();

        let deadline = states.begin_retire(&config);

        let background = config.color_theme.background().get_color();
        for state in states.elements.values() {
            let actual = state.color.last();
            assert!(
                actual
                    .iter()
                    .zip(background)
                    .all(|(a, b)| (a - b).abs() < 1e-5),
                "border color: {actual:?}, expected: {background:?}"
            );
        }
        let duration = config.char_easings.max_cpu_easing_duration().as_millis() as u32;
        assert!(deadline >= before + duration);
    }
}
