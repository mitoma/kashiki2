use std::{
    collections::{BTreeMap, HashSet},
    ops::Range,
};

use glam::{Quat, Vec3};
use log::info;
use text_buffer::action::EditorOperation;

use font_rasterizer::{
    context::{StateContext, WindowSize},
    glyph_instances::GlyphInstances,
    glyph_vertex_buffer::Direction,
    vector_instances::VectorInstances,
};

use crate::{
    camera::{Camera, CameraAdjustment, CameraController, CameraOperation},
    layout_engine::{Model, ModelOperation, ModelOperationResult, World, world::RemovedModelType},
    to_ndc_position,
};

pub struct DefaultWorld {
    camera: Camera,
    /* modal に移行する直前のカメラの位置 */
    pre_camera: Option<([f32; 3], [f32; 3])>,
    camera_controller: CameraController,
    models: Vec<Box<dyn Model>>,
    modal_models: Vec<Box<dyn Model>>,
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

    fn get_current_mut(&mut self) -> Option<&mut Box<dyn Model>> {
        self.modal_models
            .last_mut()
            .or_else(|| self.models.get_mut(self.focus))
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
        models
            .iter()
            .chain(removed_models.iter())
            .cloned()
            .collect()
    }

    #[inline]
    fn vector_instances_inner(&self) -> Vec<&VectorInstances<String>> {
        let models: Vec<&VectorInstances<String>> =
            Self::to_vector_instances(&self.models[self.get_surrounding_model_range()]);
        let removed_models: Vec<&VectorInstances<String>> =
            Self::to_vector_instances(&self.removed_models);
        models
            .iter()
            .chain(removed_models.iter())
            .cloned()
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

    fn add_modal(&mut self, model: Box<dyn Model>) {
        self.modal_models.push(model);
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
        let Some(model) = self.modal_models.last() else {
            return;
        };
        self.camera_controller
            .look_at(&mut self.camera, model.as_ref(), adjustment);
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

    fn modal_instances(&self) -> (Vec<&GlyphInstances>, Vec<&VectorInstances<String>>) {
        (
            Self::to_glyph_instances(&self.modal_models),
            Self::to_vector_instances(&self.modal_models),
        )
    }

    fn update(&mut self, context: &StateContext) {
        if self.direction != context.global_direction {
            self.direction = context.global_direction;
            self.models.iter_mut().for_each(|m| {
                m.model_operation(&ModelOperation::ChangeDirection(Some(self.direction)));
            });
            self.world_updated = true;
        }

        let range = if self.world_updated {
            0..self.models.len()
        } else {
            self.get_surrounding_model_range()
        };
        for model in self.models[range].iter_mut() {
            model.update(context);
        }
        for model in self.removed_models.iter_mut() {
            model.update(context);
        }
        for model in self.modal_models.iter_mut() {
            model.update(context);
        }
        self.removed_models.retain(|m| m.in_animation());

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
        if !self.modal_models.is_empty() {
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
        if !self.modal_models.is_empty() {
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
        if !self.modal_models.is_empty() {
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
        let (mut removed_model, removed_model_type) = self
            .modal_models
            .pop()
            .map(|m| (m, RemovedModelType::Modal))
            .unwrap_or_else(|| (self.models.remove(self.focus), RemovedModelType::Normal));
        let (x, y, z) = removed_model.position().into();
        removed_model.set_position(Vec3::new(x, y - 5.0, z));
        self.removed_models.push(removed_model);

        if self.modal_models.is_empty() && self.pre_camera.is_some() {
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
                for model in world.modal_models.iter_mut() {
                    model.set_position([focus_x, focus_y, focus_z].into());
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
