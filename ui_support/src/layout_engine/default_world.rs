use std::{
    collections::{BTreeMap, HashSet},
    iter::chain,
    ops::Range,
};

use glam::{Quat, Vec3};
use log::info;
use serde::Serialize;
use text_buffer::action::EditorOperation;

use font_rasterizer::{
    context::WindowSize, glyph_instances::GlyphInstances, glyph_vertex_buffer::Direction,
    vector_instances::VectorInstances,
};
use web_time::Duration;

use crate::ui_context::UiContext;

use crate::{
    camera::{Camera, CameraAdjustment, CameraController, CameraOperation},
    easing_value::EasingPointN,
    layout_engine::{
        DebugModelNode, Model, ModelOperation, ModelOperationResult, ModelRenderBatch,
        ModelSurfaceStyle, World, world::RemovedModelType,
    },
    to_ndc_position,
};

#[derive(Debug, Clone, Serialize)]
pub struct DebugWorldSnapshot {
    pub layout: &'static str,
    pub focus: usize,
    pub direction: &'static str,
    pub camera_target: [f32; 3],
    pub camera_eye: [f32; 3],
    pub models: Vec<DebugModelNode>,
    pub modal_models: Vec<DebugModelNode>,
    pub removed_models: Vec<DebugModelNode>,
}

fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Horizontal => "horizontal",
        Direction::Vertical => "vertical",
    }
}

fn world_layout_name(layout: &WorldLayout) -> &'static str {
    match layout {
        WorldLayout::Liner => "liner",
        WorldLayout::Circle => "circle",
    }
}

struct ModalModel {
    model: Box<dyn Model>,
    opacity: EasingPointN<1>,
    closing: bool,
}

impl ModalModel {
    fn new(model: Box<dyn Model>) -> Self {
        let mut opacity = EasingPointN::new([1.0]);
        opacity.update_duration_and_easing_func(
            Duration::from_millis(500),
            nenobi::functions::sin_in_out,
        );
        Self {
            model,
            opacity,
            closing: false,
        }
    }

    fn close(&mut self) {
        self.closing = true;
        self.opacity.update([0.0]);
    }

    fn opacity(&self) -> f32 {
        let [opacity] = self.opacity.current();
        opacity
    }

    fn fade_finished(&self) -> bool {
        self.closing && !self.opacity.in_animation()
    }
}

pub struct DefaultWorld {
    camera: Camera,
    /* modal に移行する直前のカメラの位置 */
    pre_camera: Option<([f32; 3], [f32; 3])>,
    camera_controller: CameraController,
    models: Vec<Box<dyn Model>>,
    modal_models: Vec<ModalModel>,
    removed_models: Vec<Box<dyn Model>>,
    focus: usize,
    world_updated: bool,
    direction: Direction,
    layout: WorldLayout,
}

impl DefaultWorld {
    pub fn new(window_size: WindowSize) -> Self {
        Self {
            camera: Camera::basic(window_size),
            pre_camera: None,
            camera_controller: CameraController::new(5.0),
            models: Vec::new(),
            modal_models: Vec::new(),
            removed_models: Vec::new(),
            focus: 0,
            world_updated: true,
            direction: Direction::Horizontal,
            layout: WorldLayout::Liner,
        }
    }

    // 現在フォーカスしているモデルのインデックスを返す
    pub fn focus(&self) -> usize {
        self.focus
    }

    fn get_current_mut(&mut self) -> Option<&mut Box<dyn Model>> {
        self.modal_models
            .iter_mut()
            .rev()
            .find(|modal| !modal.closing)
            .map(|modal| &mut modal.model)
            .or_else(|| self.models.get_mut(self.focus))
    }

    fn model_update_range(&mut self, calculator_updated: bool) -> Range<usize> {
        self.world_updated |= calculator_updated;
        if self.world_updated {
            0..self.models.len()
        } else {
            self.get_surrounding_model_range()
        }
    }

    fn get_surrounding_model_range(&self) -> Range<usize> {
        let around = 5;
        let min = self.focus.saturating_sub(around);
        let max = if self.focus + around < self.models.len() {
            self.focus + around
        } else {
            self.models.len()
        };
        min..max
    }

    #[inline]
    fn to_glyph_instances(models: &[Box<dyn Model>]) -> Vec<&GlyphInstances> {
        models.iter().flat_map(|m| m.glyph_instances()).collect()
    }

    #[inline]
    fn to_vector_instances(models: &[Box<dyn Model>]) -> Vec<&VectorInstances<String>> {
        models.iter().flat_map(|m| m.vector_instances()).collect()
    }

    #[inline]
    fn glyph_instances_inner(&self) -> Vec<&GlyphInstances> {
        let models: Vec<&GlyphInstances> =
            Self::to_glyph_instances(&self.models[self.get_surrounding_model_range()]);
        let removed_models: Vec<&GlyphInstances> = Self::to_glyph_instances(&self.removed_models);
        chain(models, removed_models).collect()
    }

    #[inline]
    fn vector_instances_inner(&self) -> Vec<&VectorInstances<String>> {
        let models: Vec<&VectorInstances<String>> =
            Self::to_vector_instances(&self.models[self.get_surrounding_model_range()]);
        let removed_models: Vec<&VectorInstances<String>> =
            Self::to_vector_instances(&self.removed_models);
        chain(models, removed_models).collect()
    }

    fn render_batches_inner(&self) -> Vec<ModelRenderBatch<'_>> {
        let models = &self.models[self.get_surrounding_model_range()];
        // 半透明の描画を正しく重ねるため、視線方向で奥にあるモデルから順に描画する
        let view_depth = self.camera.view_depth_fn();
        let mut models: Vec<(f32, &dyn Model)> = models
            .iter()
            .chain(self.removed_models.iter())
            .map(|model| (view_depth(model.position()), model.as_ref()))
            .collect();
        models.sort_by(|(a, _), (b, _)| b.total_cmp(a));
        models
            .into_iter()
            .map(|(_, model)| (model, 1.0))
            // モーダルは奥行きによらず通常モデルより手前に重ねる。
            .chain(
                self.modal_models
                    .iter()
                    .map(|modal| (modal.model.as_ref(), modal.opacity())),
            )
            .map(|(model, opacity)| ModelRenderBatch {
                surface_instances: model.surface_instances(),
                vector_instances: model.vector_instances(),
                glyph_instances: model.glyph_instances(),
                opacity,
            })
            .collect()
    }
}

impl World for DefaultWorld {
    fn add(&mut self, model: Box<dyn Model>) {
        self.models.push(model);
        self.world_updated = true;
    }

    fn add_next(&mut self, model: Box<dyn Model>) {
        self.models.insert(self.focus + 1, model);
        self.world_updated = true;
    }

    fn add_modal(&mut self, mut model: Box<dyn Model>) {
        model.set_surface_style(ModelSurfaceStyle::ThemeBackground { alpha: 0.9 });
        self.modal_models.push(ModalModel::new(model));
        self.pre_camera = Some(self.camera.target_and_eye());
        self.world_updated = true;
    }

    fn re_layout(&mut self) {
        self.layout.clone().layout(self);
    }

    fn model_length(&self) -> usize {
        self.models.len()
    }

    fn look_at(&mut self, model_index: usize, adjustment: CameraAdjustment) {
        let Some(model) = self.models.get(model_index) else {
            return;
        };
        self.focus = model_index;
        self.camera_controller
            .look_at(&mut self.camera, model.as_ref(), adjustment);
        self.camera_controller.update_camera(&mut self.camera);
    }

    fn look_modal(&mut self, adjustment: CameraAdjustment) {
        let Some(modal) = self.modal_models.iter().rev().find(|modal| !modal.closing) else {
            return;
        };
        self.camera_controller
            .look_at(&mut self.camera, modal.model.as_ref(), adjustment);
        self.camera_controller.update_camera(&mut self.camera);
    }

    fn camera(&self) -> &Camera {
        &self.camera
    }

    fn glyph_instances(&self) -> Vec<&GlyphInstances> {
        self.glyph_instances_inner()
    }

    fn vector_instances(&self) -> Vec<&VectorInstances<String>> {
        self.vector_instances_inner()
    }

    fn render_batches(&self) -> Vec<ModelRenderBatch<'_>> {
        self.render_batches_inner()
    }

    fn has_modal(&self) -> bool {
        self.modal_models.iter().any(|modal| !modal.closing)
    }

    fn modal_instances(&self) -> (Vec<&GlyphInstances>, Vec<&VectorInstances<String>>) {
        (
            self.modal_models
                .iter()
                .filter(|modal| !modal.closing)
                .flat_map(|modal| modal.model.glyph_instances())
                .collect(),
            self.modal_models
                .iter()
                .filter(|modal| !modal.closing)
                .flat_map(|modal| modal.model.vector_instances())
                .collect(),
        )
    }

    fn update(&mut self, context: &UiContext) {
        if self.direction != context.global_direction() {
            self.direction = context.global_direction();
            self.models.iter_mut().for_each(|m| {
                m.model_operation(&ModelOperation::ChangeDirection(Some(self.direction)));
            });
            self.world_updated = true;
        }

        let range = self.model_update_range(context.calculator_updated());
        for model in self.models[range].iter_mut() {
            model.update(context);
        }
        for model in self.removed_models.iter_mut() {
            model.update(context);
        }
        for modal in self.modal_models.iter_mut() {
            modal.model.update(context);
        }
        self.removed_models.retain(|m| m.in_animation());
        self.modal_models.retain(|modal| !modal.fade_finished());

        if self.world_updated {
            self.re_layout();
            self.look_current(CameraAdjustment::NoCare);
            self.world_updated = false;
        }
    }

    fn change_window_size(&mut self, window_size: WindowSize) {
        self.camera_controller
            .update_camera_aspect(&mut self.camera, window_size);
    }

    fn camera_operation(&mut self, camera_operation: CameraOperation) {
        self.camera_controller.process(&camera_operation);
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_controller.reset_state();
    }

    fn look_current(&mut self, adjustment: CameraAdjustment) {
        // modal の場合はフォーカスを移動させない
        if self.has_modal() {
            self.look_modal(adjustment);
        } else {
            self.look_at(self.focus, adjustment)
        }
    }

    fn look_next(&mut self, adjustment: CameraAdjustment) {
        // model がない場合は何もしない
        if self.model_length() == 0 {
            return;
        }

        // modal の場合はフォーカスを移動させない
        if self.has_modal() {
            self.look_modal(adjustment);
            return;
        }
        let next = (self.focus + 1) % self.model_length();
        self.look_at(next, adjustment)
    }

    fn look_prev(&mut self, adjustment: CameraAdjustment) {
        // model がない場合は何もしない
        if self.model_length() == 0 {
            return;
        }

        // modal の場合はフォーカスを移動させない
        if self.has_modal() {
            self.look_modal(adjustment);
            return;
        }
        let prev = if self.focus == 0 {
            self.model_length() - 1
        } else {
            self.focus - 1
        };
        self.look_at(prev, adjustment)
    }

    fn editor_operation(&mut self, op: &EditorOperation) {
        self.world_updated = true;
        if let Some(model) = self.get_current_mut() {
            model.editor_operation(op);
        }
    }

    fn model_operation(&mut self, op: &ModelOperation) {
        if let Some(model) = self.get_current_mut() {
            match model.model_operation(op) {
                ModelOperationResult::NoCare => {}
                ModelOperationResult::RequireReLayout => {
                    self.world_updated = true;
                }
                ModelOperationResult::RequireReLayoutAndFit => {
                    self.world_updated = true;
                    self.look_current(CameraAdjustment::FitBoth);
                }
            }
        }
    }

    fn current_string(&self) -> String {
        self.models[self.focus].to_string()
    }

    fn chars(&self) -> HashSet<char> {
        self.models
            .iter()
            .flat_map(|m| m.to_string().chars().collect::<HashSet<char>>())
            .collect()
    }

    fn strings(&self) -> Vec<String> {
        self.models.iter().map(|m| m.to_string()).collect()
    }

    fn remove_current(&mut self) -> RemovedModelType {
        self.world_updated = true;
        let removed_model_type = if let Some(modal) = self
            .modal_models
            .iter_mut()
            .rev()
            .find(|modal| !modal.closing)
        {
            modal.close();
            RemovedModelType::Modal
        } else {
            let mut removed_model = self.models.remove(self.focus);
            let (x, y, z) = removed_model.position().into();
            removed_model.set_position(Vec3::new(x, y - 5.0, z));
            self.removed_models.push(removed_model);
            RemovedModelType::Normal
        };

        if !self.has_modal() && self.pre_camera.is_some() {
            // modal から戻るときはカメラを元に戻す
            if let Some((target, eye)) = self.pre_camera.take() {
                self.camera_operation(CameraOperation::CangeTargetAndEye(
                    target.into(),
                    eye.into(),
                ));
            }
        }

        removed_model_type
    }

    fn swap_next(&mut self) {
        self.world_updated = true;
        let has_next = self.focus + 1 < self.model_length();
        if !has_next {
            return;
        }
        self.models.swap(self.focus, self.focus + 1);
        self.look_at(self.focus + 1, CameraAdjustment::NoCare);
    }

    fn swap_prev(&mut self) {
        self.world_updated = true;
        let has_prev = self.focus > 0;
        if !has_prev {
            return;
        }
        self.models.swap(self.focus, self.focus - 1);
        self.re_layout();
        self.look_at(self.focus - 1, CameraAdjustment::NoCare);
    }

    fn move_to_position(&mut self, x_ratio: f32, y_ratio: f32) {
        let mut distance_map: BTreeMap<usize, f32> = BTreeMap::new();

        for (idx, model) in self.models.iter().enumerate() {
            let (ndc_x, ndc_y) = to_ndc_position(model.as_ref(), &self.camera);
            let distance = (x_ratio - ndc_x).abs().powf(2.0) + (y_ratio - ndc_y).abs().powf(2.0);
            distance_map.insert(idx, distance);
        }

        let min_distance = distance_map
            .iter()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap());

        if let Some((idx, _)) = min_distance {
            if idx != &self.focus {
                self.look_at(*idx, CameraAdjustment::NoCare);
            } else {
                self.model_operation(&ModelOperation::MoveToClick(
                    x_ratio,
                    y_ratio,
                    self.camera.build_view_projection_matrix(),
                ));
            }
        }
    }

    fn change_layout(&mut self, layout: WorldLayout) {
        if self.layout == layout {
            return;
        }
        self.layout = layout;
        self.world_updated = true;
    }
    fn layout(&self) -> &WorldLayout {
        &self.layout
    }

    fn change_char_easings_preset(&mut self, preset: crate::ui_context::CharEasingsPreset) {
        self.models.iter_mut().for_each(|m| {
            m.set_easing_preset(preset);
        });
        self.modal_models.iter_mut().for_each(|modal| {
            if !modal.closing {
                modal.model.set_easing_preset(preset);
            }
        });
    }

    fn debug_snapshot(&self) -> DebugWorldSnapshot {
        let (camera_target, camera_eye) = self.camera.target_and_eye();
        DebugWorldSnapshot {
            layout: world_layout_name(&self.layout),
            focus: self.focus,
            direction: direction_name(self.direction),
            camera_target,
            camera_eye,
            models: self
                .models
                .iter()
                .map(|model| model.debug_node(&self.camera))
                .collect(),
            modal_models: self
                .modal_models
                .iter()
                .filter(|modal| !modal.closing)
                .map(|modal| modal.model.debug_node(&self.camera))
                .collect(),
            removed_models: self
                .removed_models
                .iter()
                .map(|model| model.as_ref())
                .chain(
                    self.modal_models
                        .iter()
                        .filter(|modal| modal.closing)
                        .map(|modal| modal.model.as_ref()),
                )
                .map(|model| model.debug_node(&self.camera))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorldLayout {
    Liner,
    Circle,
}

const INTERVAL: f32 = 5.0;
impl WorldLayout {
    pub fn next(&self) -> Self {
        match self {
            WorldLayout::Liner => WorldLayout::Circle,
            WorldLayout::Circle => WorldLayout::Liner,
        }
    }

    fn layout(&self, world: &mut DefaultWorld) {
        match self {
            WorldLayout::Liner => {
                let mut position = 0.0;
                for (idx, model) in world.models.iter_mut().enumerate() {
                    let (w, h) = model.bound();
                    info!("w: {}, h: {}, idx:{}", w, h, idx);

                    let rotation = Quat::IDENTITY;
                    model.set_rotation(rotation);

                    match world.direction {
                        Direction::Horizontal => {
                            position += w / 2.0;
                            model.set_position((position, -h / 2.0, 0.0).into());
                            position += w / 2.0 + INTERVAL;
                        }
                        Direction::Vertical => {
                            position -= h / 2.0;
                            model.set_position((-w / 2.0, position, 0.0).into());
                            position -= h / 2.0 + INTERVAL;
                        }
                    }
                }
                let [focus_x, focus_y, focus_z] = world
                    .models
                    .get(world.focus)
                    .map(|m| m.focus_position())
                    .unwrap_or([0.0, 0.0, 0.0].into())
                    .into();
                for modal in world.modal_models.iter_mut().filter(|modal| !modal.closing) {
                    modal.model.set_position([focus_x, focus_y, focus_z].into());
                }
            }
            WorldLayout::Circle => {
                match world.direction {
                    Direction::Horizontal => {
                        // すべてのモデルの幅の合計
                        let all_width: f32 =
                            world.models.iter().map(|m| m.bound().0 + INTERVAL).sum();
                        // all_width を円周とみなして半径を求める
                        let radius = all_width / (2.0 * std::f32::consts::PI);

                        let mut x_position = 0.0;
                        for (idx, model) in world.models.iter_mut().enumerate() {
                            let (w, h) = model.bound();
                            x_position += w / 2.0;
                            info!("w: {}, h: {}, idx:{}", w, h, idx);
                            let r = (x_position / all_width) * 2.0 * std::f32::consts::PI;
                            model.set_position(
                                (r.sin() * radius, -h / 2.0, -(r.cos() - 1.0) * radius).into(),
                            );
                            x_position += w / 2.0 + INTERVAL;

                            let rotation = Quat::from_axis_angle(Vec3::Y, -r);
                            model.set_rotation(rotation);
                        }
                    }
                    Direction::Vertical => {
                        // すべてのモデルの幅の合計
                        let all_height: f32 =
                            world.models.iter().map(|m| m.bound().1 + INTERVAL).sum();
                        // all_width を円周とみなして半径を求める
                        let radius = all_height / (2.0 * std::f32::consts::PI);

                        let mut y_position = 0.0;
                        for (idx, model) in world.models.iter_mut().enumerate() {
                            let (w, h) = model.bound();
                            y_position += h / 2.0;
                            info!("w: {}, h: {}, idx:{}", w, h, idx);
                            let r = (y_position / all_height) * 2.0 * std::f32::consts::PI;
                            model.set_position(
                                (-w / 2.0, -r.sin() * radius, -(r.cos() + 1.0) * radius).into(),
                            );
                            y_position += h / 2.0 + INTERVAL;

                            let rotation = Quat::from_axis_angle(Vec3::X, -r);
                            model.set_rotation(rotation);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod font_change_tests {
    use super::*;

    #[test]
    fn modal_models_share_render_batches_and_use_translucent_surfaces() {
        let mut world = DefaultWorld::new(WindowSize::new(800, 600));
        let mut normal = crate::ui::TextEdit::default();
        normal.set_position(Vec3::new(1.0, 2.0, 3.0));
        world.add(Box::new(normal));
        assert!(!world.has_modal());
        assert_eq!(world.render_batches().len(), 1);

        for _ in 0..2 {
            world.add_modal(Box::new(crate::ui::TextEdit::default()));
        }

        assert!(world.has_modal());
        assert_eq!(world.render_batches().len(), 3);
        assert_eq!(world.models[0].surface_style(), ModelSurfaceStyle::None);
        for modal in &world.modal_models {
            let ModelSurfaceStyle::ThemeBackground { alpha } = modal.model.surface_style() else {
                panic!("modal must use the theme background");
            };
            assert!(alpha > 0.0 && alpha < 1.0);
        }

        let last_position = world.modal_models[1].model.last_position();
        assert!(matches!(world.remove_current(), RemovedModelType::Modal));
        assert!(world.has_modal());
        assert_eq!(world.modal_models[1].model.last_position(), last_position);
        assert!(world.modal_models[1].closing);
        assert_eq!(world.modal_models[1].opacity.last(), [0.0]);
        assert_eq!(world.render_batches().len(), 3);
        assert!(matches!(world.remove_current(), RemovedModelType::Modal));
        assert!(!world.has_modal());
        assert!(world.modal_models.iter().all(|modal| modal.closing));
        assert!(world.removed_models.is_empty());
        assert!(world.modal_instances().0.is_empty());
        assert_eq!(
            world.get_current_mut().unwrap().last_position(),
            Vec3::new(1.0, 2.0, 3.0)
        );
        assert_eq!(world.render_batches().len(), 3);
        world.modal_models.clear();
        assert_eq!(world.render_batches().len(), 1);
    }

    #[test]
    fn closing_modals_keep_their_layer_when_another_modal_opens() {
        let mut world = DefaultWorld::new(WindowSize::new(800, 600));
        world.add(Box::new(crate::ui::TextEdit::default()));
        world.add_modal(Box::new(crate::ui::TextEdit::default()));
        world.add_modal(Box::new(crate::ui::TextEdit::default()));
        assert!(matches!(world.remove_current(), RemovedModelType::Modal));

        world.modal_models[1].opacity = EasingPointN::new([0.5]);
        world.add_modal(Box::new(crate::ui::TextEdit::default()));

        let batches = world.render_batches();
        assert_eq!(batches.len(), 4);
        assert_eq!(batches[0].opacity, 1.0);
        assert_eq!(batches[1].opacity, 1.0);
        assert!((batches[2].opacity - 0.5).abs() < 0.001);
        assert_eq!(batches[3].opacity, 1.0);
    }

    #[test]
    fn font_change_invalidates_world_layout() {
        let mut world = DefaultWorld::new(WindowSize::new(800, 600));
        for _ in 0..10 {
            world.add(Box::new(crate::ui::TextEdit::default()));
        }
        world.world_updated = false;

        assert_eq!(world.model_update_range(false), 0..5);
        assert!(!world.world_updated);

        assert_eq!(world.model_update_range(true), 0..10);
        assert!(world.world_updated);
        assert_eq!(world.model_update_range(false), 0..10);

        world.world_updated = false;
        assert_eq!(world.model_update_range(false), 0..5);
        assert!(!world.world_updated);
    }
}
