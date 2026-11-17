use std::{
    collections::{BTreeMap, HashSet},
    ops::Range,
};

use glam::{Quat, Vec3};
use log::info;
use serde::Serialize;
use text_buffer::action::EditorOperation;

use font_rasterizer::{
    context::WindowSize, glyph_instances::GlyphInstances, glyph_vertex_buffer::Direction,
    time::now_millis, vector_instances::VectorInstances,
};
use web_time::Duration;

use crate::ui_context::UiContext;

use crate::{
    camera::{Camera, CameraAdjustment, CameraController, CameraOperation},
    layout_engine::{
        DebugModelNode, Model, ModelOperation, ModelOperationResult, ModelRenderBatch,
        ModelSurfaceStyle, SplitError, SwitchModelError, World, world::RemovedModelType,
    },
    retiring::{FadeOut, Retiring},
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
        WorldLayout::Split(Direction::Vertical) => "split-vertical",
        WorldLayout::Split(Direction::Horizontal) => "split-horizontal",
    }
}

// モデルが退場するときのフェードの長さ
const MODEL_EXIT_DURATION: Duration = Duration::from_millis(500);
// 通常モデルが退場するときに下へずらす距離
const MODEL_EXIT_SLIDE: f32 = 5.0;

struct ModalModel {
    model: Box<dyn Model>,
    // モーダル同士の重なり順。退場中のモーダルもこの順序を保って描画する
    order: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DepartingLayer {
    Normal,
    Modal { order: usize },
}

// 退場アニメーション中のモデル。破棄時刻は Retiring が管理する
struct DepartingModel {
    model: Box<dyn Model>,
    fade: FadeOut,
    layer: DepartingLayer,
}

pub struct DefaultWorld {
    camera: Camera,
    /* modal に移行する直前のカメラの位置 */
    pre_camera: Option<([f32; 3], [f32; 3])>,
    camera_controller: CameraController,
    models: Vec<Box<dyn Model>>,
    modal_models: Vec<ModalModel>,
    next_modal_order: usize,
    departing_models: Retiring<DepartingModel>,
    focus: usize,
    world_updated: bool,
    direction: Direction,
    layout: WorldLayout,
    split_panes: Option<[usize; 2]>,
    // 分割中に直近で選択されたフィット方法。カーソル移動や編集時の再フィットに使う
    split_adjustment: CameraAdjustment,
    // 強調中のモデルの識別子と、強調前の面の設定
    focused_pane_highlight: Option<(usize, ModelSurfaceStyle)>,
}

// Box 内のモデルはヒープ上で移動しないため、アドレスを並べ替えに影響されない識別子として使う
fn model_id(model: &dyn Model) -> usize {
    (model as *const dyn Model).cast::<()>() as usize
}

impl DefaultWorld {
    pub fn new(window_size: WindowSize) -> Self {
        Self {
            camera: Camera::basic(window_size),
            pre_camera: None,
            camera_controller: CameraController::new(5.0),
            models: Vec::new(),
            modal_models: Vec::new(),
            next_modal_order: 0,
            departing_models: Retiring::default(),
            focus: 0,
            world_updated: true,
            direction: Direction::Horizontal,
            layout: WorldLayout::Liner,
            split_panes: None,
            split_adjustment: CameraAdjustment::FitBoth,
            focused_pane_highlight: None,
        }
    }

    // 現在フォーカスしているモデルのインデックスを返す
    pub fn focus(&self) -> usize {
        self.focus
    }

    /// 表示中のモデルとモーダルを全て退場させる。カメラとレイアウトは引き継ぐ
    pub fn retire_all(&mut self) {
        self.restore_focused_pane_surface();
        let visible: HashSet<usize> = self
            .visible_models()
            .map(|model| model_id(model.as_ref()))
            .collect();
        // 画面外のモデルまで退場させると描画と更新の負荷が増えるので、見えているものだけ演出する
        for mut model in std::mem::take(&mut self.models) {
            if visible.contains(&model_id(model.as_ref())) {
                let (x, y, z) = model.position().into();
                model.set_position(Vec3::new(x, y - MODEL_EXIT_SLIDE, z));
                self.depart(model, DepartingLayer::Normal);
            }
        }
        for modal in std::mem::take(&mut self.modal_models) {
            self.depart(modal.model, DepartingLayer::Modal { order: modal.order });
        }
        self.focus = 0;
        if self.split_panes.take().is_some() {
            self.layout = WorldLayout::Liner;
        }
        self.pre_camera = None;
        self.world_updated = true;
    }

    /// 表示中のモデルを退場させ、新しいモデルに差し替える
    pub fn replace_models(&mut self, models: impl IntoIterator<Item = Box<dyn Model>>) {
        self.retire_all();
        self.models.extend(models);
    }

    fn visible_models(&self) -> impl Iterator<Item = &Box<dyn Model>> {
        let range = if self.split_panes.is_some() {
            0..self.models.len()
        } else {
            self.get_surrounding_model_range()
        };
        self.models[range]
            .iter()
            .enumerate()
            .filter_map(move |(index, model)| {
                let visible = match self.split_panes {
                    Some(panes) => panes.contains(&index),
                    None => true,
                };
                visible.then_some(model)
            })
    }

    fn fit_split(&mut self, adjustment: CameraAdjustment) {
        let Some(panes) = self.split_panes else {
            return;
        };
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for index in panes {
            let model = &self.models[index];
            let (w, h) = model.bound();
            let half = Vec3::new(w / 2.0, h / 2.0, 0.0);
            min = min.min(model.last_position() - half);
            max = max.max(model.last_position() + half);
        }
        let mut center = (min + max) / 2.0;
        let bound = max - min;
        let fit_bound = (bound.x, bound.y);
        let mut anchor_bound = fit_bound;
        if self.aligns_split_carets() {
            // カーソルの高さを揃えている間は、上下方向を分割していないときと同様に
            // フォーカス中の文書の範囲で扱い、上端・下端付近ではカーソルを中央へ寄せない。
            // 倍率は高い方の文書に合わせ、カーソル移動やフォーカス切り替えで変わらないようにする
            let focused = &self.models[self.focus];
            let fit_h = panes
                .iter()
                .map(|index| self.models[*index].bound().1)
                .fold(0.0, f32::max);
            center.y = focused.last_position().y;
            anchor_bound.1 = focused.bound().1;
            self.look_at_split_geometry((bound.x, fit_h), anchor_bound, center, adjustment);
            self.center_unfocused_pane_caret();
            return;
        }
        self.look_at_split_geometry(fit_bound, anchor_bound, center, adjustment);
    }

    // フォーカスの無い文書は、カーソルが画面の中心の高さに来るように置く
    fn center_unfocused_pane_caret(&mut self) {
        let Some(panes) = self.split_panes else {
            return;
        };
        if !self.aligns_split_carets() {
            return;
        }
        let other = if panes[0] == self.focus {
            panes[1]
        } else {
            panes[0]
        };
        let target_y = self.camera.target_and_eye().0[1];
        let model = &mut self.models[other];
        let mut position = model.last_position();
        position.y = target_y - (model.focus_position().y - position.y);
        model.set_position(position);
    }

    fn look_at_split_geometry(
        &mut self,
        fit_bound: (f32, f32),
        anchor_bound: (f32, f32),
        center: Vec3,
        adjustment: CameraAdjustment,
    ) {
        // 画面からはみ出す方向はフォーカス中の文書のカーソルに追従させる
        let focus_position = self.models[self.focus].focus_position();
        self.camera_controller.look_at_geometry(
            &mut self.camera,
            fit_bound,
            anchor_bound,
            center,
            focus_position,
            Quat::IDENTITY,
            adjustment,
        );
        self.camera_controller.update_camera(&mut self.camera);
    }

    // 左右分割で全体表示以外のときは、2文書のカーソルの高さを揃える
    fn aligns_split_carets(&self) -> bool {
        self.layout == WorldLayout::Split(Direction::Vertical)
            && self.split_adjustment != CameraAdjustment::FitBoth
    }

    fn remap_split_panes(&mut self, mut remap: impl FnMut(usize) -> usize) {
        if let Some(panes) = &mut self.split_panes {
            for index in panes {
                *index = remap(*index);
            }
        }
    }

    // 分割中はフォーカスがある文書だけを強調し、強調を外すときは元の面の設定に戻す
    fn sync_focused_pane_surface(&mut self) {
        let target = self
            .split_panes
            .and_then(|_| self.models.get(self.focus))
            .map(|model| model_id(model.as_ref()));
        if self.focused_pane_highlight.map(|(id, _)| id) == target {
            return;
        }
        self.restore_focused_pane_surface();
        if let Some(id) = target {
            let model = &mut self.models[self.focus];
            self.focused_pane_highlight = Some((id, model.surface_style()));
            model.set_surface_style(FOCUSED_PANE_SURFACE);
        }
    }

    fn restore_focused_pane_surface(&mut self) {
        let Some((id, original)) = self.focused_pane_highlight.take() else {
            return;
        };
        if let Some(model) = self
            .models
            .iter_mut()
            .find(|model| model_id(model.as_ref()) == id)
        {
            model.set_surface_style(original);
        }
    }

    fn get_current_mut(&mut self) -> Option<&mut Box<dyn Model>> {
        self.modal_models
            .last_mut()
            .map(|modal| &mut modal.model)
            .or_else(|| self.models.get_mut(self.focus))
    }

    // モデルを退場させ、フェードが終わる時刻に破棄されるようにする
    fn depart(&mut self, model: Box<dyn Model>, layer: DepartingLayer) {
        let fade = FadeOut::start(MODEL_EXIT_DURATION);
        let deadline = fade.deadline();
        self.departing_models
            .retire(DepartingModel { model, fade, layer }, deadline);
    }

    fn departing_normal_models(&self) -> impl Iterator<Item = &DepartingModel> {
        self.departing_models
            .iter()
            .filter(|departing| departing.layer == DepartingLayer::Normal)
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
    fn glyph_instances_inner(&self) -> Vec<&GlyphInstances> {
        self.visible_models()
            .chain(self.departing_normal_models().map(|d| &d.model))
            .flat_map(|model| model.glyph_instances())
            .collect()
    }

    #[inline]
    fn vector_instances_inner(&self) -> Vec<&VectorInstances<String>> {
        self.visible_models()
            .chain(self.departing_normal_models().map(|d| &d.model))
            .flat_map(|model| model.vector_instances())
            .collect()
    }

    fn render_batches_inner(&self) -> Vec<ModelRenderBatch<'_>> {
        // 半透明の描画を正しく重ねるため、視線方向で奥にあるモデルから順に描画する
        let view_depth = self.camera.view_depth_fn();
        let mut models: Vec<(f32, &dyn Model, f32)> = self
            .visible_models()
            .map(|model| (model.as_ref(), 1.0))
            .chain(
                self.departing_normal_models()
                    .map(|departing| (departing.model.as_ref(), departing.fade.opacity())),
            )
            .map(|(model, opacity)| (view_depth(model.position()), model, opacity))
            .collect();
        models.sort_by(|(a, _, _), (b, _, _)| b.total_cmp(a));

        // モーダルは奥行きによらず通常モデルより手前に、開いた順に重ねる。
        // 退場中のモーダルも開いた時の順序を保つ。
        let mut modals: Vec<(usize, &dyn Model, f32)> = self
            .modal_models
            .iter()
            .map(|modal| (modal.order, modal.model.as_ref(), 1.0))
            .chain(
                self.departing_models
                    .iter()
                    .filter_map(|departing| match departing.layer {
                        DepartingLayer::Modal { order } => {
                            Some((order, departing.model.as_ref(), departing.fade.opacity()))
                        }
                        DepartingLayer::Normal => None,
                    }),
            )
            .collect();
        modals.sort_by_key(|(order, _, _)| *order);

        models
            .into_iter()
            .map(|(_, model, opacity)| (model, opacity))
            .chain(
                modals
                    .into_iter()
                    .map(|(_, model, opacity)| (model, opacity)),
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
        let index = self.focus + 1;
        self.models.insert(index, model);
        self.remap_split_panes(|pane| if pane >= index { pane + 1 } else { pane });
        self.world_updated = true;
    }

    fn add_modal(&mut self, mut model: Box<dyn Model>) {
        model.set_surface_style(ModelSurfaceStyle::ThemeBackground { alpha: 0.9 });
        let order = self.next_modal_order;
        self.next_modal_order += 1;
        self.modal_models.push(ModalModel { model, order });
        self.pre_camera = Some(self.camera.target_and_eye());
        self.world_updated = true;
    }

    fn re_layout(&mut self) {
        self.layout.clone().layout(self);
    }

    fn model_length(&self) -> usize {
        self.models.len()
    }

    fn focus(&self) -> usize {
        self.focus
    }

    fn split(&mut self, direction: Direction, other_model: usize) -> Result<(), SplitError> {
        if self.has_modal() {
            return Err(SplitError::ModalOpen);
        }
        if self.models.get(other_model).is_none() || self.models.get(self.focus).is_none() {
            return Err(SplitError::InvalidModel(other_model));
        }
        if self.focus == other_model {
            return Err(SplitError::SameModel);
        }
        let panes = match self.split_panes {
            Some(mut panes) => {
                let other_slot = if panes[0] == self.focus { 1 } else { 0 };
                panes[other_slot] = other_model;
                panes
            }
            None => {
                self.split_adjustment = CameraAdjustment::FitBoth;
                [self.focus, other_model]
            }
        };
        self.layout = WorldLayout::Split(direction);
        self.split_panes = Some(panes);
        self.world_updated = true;
        self.re_layout();
        self.fit_split(self.split_adjustment);
        Ok(())
    }

    fn look_other_pane(&mut self) {
        if self.has_modal() {
            return;
        }
        if let Some(panes) = self.split_panes {
            let other = if panes[0] == self.focus {
                panes[1]
            } else {
                panes[0]
            };
            self.look_at(other, CameraAdjustment::NoCare);
        }
    }

    fn switch_model(&mut self, model_index: usize) -> Result<(), SwitchModelError> {
        if self.has_modal() {
            return Err(SwitchModelError::ModalOpen);
        }
        if model_index >= self.models.len() {
            return Err(SwitchModelError::InvalidModel(model_index));
        }
        if model_index == self.focus {
            return Ok(());
        }
        if let Some(panes) = &mut self.split_panes
            && panes.contains(&model_index)
        {
            // もう一方の Pane の文書を選んだときは、2つの Pane の文書を入れ替える
            panes.swap(0, 1);
            self.focus = model_index;
            self.world_updated = true;
            self.re_layout();
            self.fit_split(self.split_adjustment);
            return Ok(());
        }
        self.look_at(model_index, CameraAdjustment::NoCare);
        Ok(())
    }

    fn look_at(&mut self, model_index: usize, adjustment: CameraAdjustment) {
        if let Some(panes) = &mut self.split_panes
            && model_index < self.models.len()
        {
            let changed = !panes.contains(&model_index);
            if changed {
                let slot = if panes[0] == self.focus { 0 } else { 1 };
                panes[slot] = model_index;
                self.world_updated = true;
            }
            // 現在の文書へのフィット指定（reset-zoom, fit-width, fit-height）は以降の再フィットにも使う
            if model_index == self.focus {
                match adjustment {
                    CameraAdjustment::FitBoth | CameraAdjustment::FitBothAndCentering => {
                        self.split_adjustment = CameraAdjustment::FitBoth;
                    }
                    CameraAdjustment::FitWidth | CameraAdjustment::FitHeight => {
                        self.split_adjustment = adjustment;
                    }
                    CameraAdjustment::NoCare => {}
                }
            }
            self.focus = model_index;
            self.re_layout();
            let adjustment = if adjustment == CameraAdjustment::NoCare && !self.world_updated {
                CameraAdjustment::NoCare
            } else {
                self.split_adjustment
            };
            self.fit_split(adjustment);
            return;
        }
        let Some(model) = self.models.get(model_index) else {
            return;
        };
        self.focus = model_index;
        self.camera_controller
            .look_at(&mut self.camera, model.as_ref(), adjustment);
        self.camera_controller.update_camera(&mut self.camera);
    }

    fn look_modal(&mut self, adjustment: CameraAdjustment) {
        let Some(modal) = self.modal_models.last() else {
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
        !self.modal_models.is_empty()
    }

    fn modal_instances(&self) -> (Vec<&GlyphInstances>, Vec<&VectorInstances<String>>) {
        (
            self.modal_models
                .iter()
                .flat_map(|modal| modal.model.glyph_instances())
                .collect(),
            self.modal_models
                .iter()
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

        self.sync_focused_pane_surface();
        let range = self.model_update_range(context.calculator_updated());
        for model in self.models[range.clone()].iter_mut() {
            model.update(context);
        }
        if let Some(panes) = self.split_panes {
            for index in panes {
                if !range.contains(&index) {
                    self.models[index].update(context);
                }
            }
        }
        for (_, departing) in self.departing_models.iter_mut() {
            departing.model.update(context);
        }
        for modal in self.modal_models.iter_mut() {
            modal.model.update(context);
        }
        self.departing_models.sweep(now_millis(), |_, _| {});

        if self.world_updated {
            self.re_layout();
            self.look_current(CameraAdjustment::NoCare);
            self.world_updated = false;
        }
    }

    fn change_window_size(&mut self, window_size: WindowSize) {
        self.camera_controller
            .update_camera_aspect(&mut self.camera, window_size);
        if self.split_panes.is_some() {
            self.world_updated = true;
        }
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
                    // 分割中は直近のフィット方法を維持する
                    let adjustment = if self.split_panes.is_some() {
                        CameraAdjustment::NoCare
                    } else {
                        CameraAdjustment::FitBoth
                    };
                    self.look_current(adjustment);
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
        let removed_model_type = if let Some(modal) = self.modal_models.pop() {
            self.depart(modal.model, DepartingLayer::Modal { order: modal.order });
            RemovedModelType::Modal
        } else {
            // 破棄されるモデルの識別子を残さないよう、削除前に強調を外す
            self.restore_focused_pane_surface();
            let mut removed_model = self.models.remove(self.focus);
            let (x, y, z) = removed_model.position().into();
            removed_model.set_position(Vec3::new(x, y - MODEL_EXIT_SLIDE, z));
            self.depart(removed_model, DepartingLayer::Normal);
            if let Some(panes) = self.split_panes {
                if self.models.len() < 2 {
                    self.split_panes = None;
                    self.layout = WorldLayout::Liner;
                    self.focus = 0;
                } else {
                    let removed = self.focus;
                    let surviving = if panes[0] == removed {
                        panes[1]
                    } else {
                        panes[0]
                    };
                    let surviving = if surviving > removed {
                        surviving - 1
                    } else {
                        surviving
                    };
                    let replacement = (removed..self.models.len())
                        .chain(0..removed.min(self.models.len()))
                        .find(|index| *index != surviving)
                        .unwrap();
                    self.split_panes = Some(panes.map(|index| {
                        if index == removed {
                            replacement
                        } else if index > removed {
                            index - 1
                        } else {
                            index
                        }
                    }));
                    self.focus = replacement;
                }
            }
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
        let focus = self.focus;
        self.remap_split_panes(|index| {
            if index == focus {
                focus + 1
            } else if index == focus + 1 {
                focus
            } else {
                index
            }
        });
        self.focus += 1;
        self.look_at(self.focus, CameraAdjustment::NoCare);
    }

    fn swap_prev(&mut self) {
        self.world_updated = true;
        let has_prev = self.focus > 0;
        if !has_prev {
            return;
        }
        self.models.swap(self.focus, self.focus - 1);
        let focus = self.focus;
        self.remap_split_panes(|index| {
            if index == focus {
                focus - 1
            } else if index == focus - 1 {
                focus
            } else {
                index
            }
        });
        self.focus -= 1;
        self.re_layout();
        self.look_at(self.focus, CameraAdjustment::NoCare);
    }

    fn move_to_position(&mut self, x_ratio: f32, y_ratio: f32) {
        let mut distance_map: BTreeMap<usize, f32> = BTreeMap::new();

        for (idx, model) in self.models.iter().enumerate() {
            if self.split_panes.is_some_and(|panes| !panes.contains(&idx)) {
                continue;
            }
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
        match layout {
            WorldLayout::Split(_) if self.split_panes.is_none() && self.models.len() >= 2 => {
                self.split_panes = Some([self.focus, (self.focus + 1) % self.models.len()]);
            }
            WorldLayout::Split(_) => {}
            _ => self.split_panes = None,
        }
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
            modal.model.set_easing_preset(preset);
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
                .map(|modal| modal.model.debug_node(&self.camera))
                .collect(),
            removed_models: {
                // 通常モデルを先に、モーダルは開いた順に並べる
                let mut departing: Vec<&DepartingModel> = self.departing_models.iter().collect();
                departing.sort_by_key(|departing| match departing.layer {
                    DepartingLayer::Normal => (0, 0),
                    DepartingLayer::Modal { order } => (1, order),
                });
                departing
                    .into_iter()
                    .map(|departing| departing.model.debug_node(&self.camera))
                    .collect()
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorldLayout {
    Liner,
    Circle,
    Split(Direction),
}

const INTERVAL: f32 = 5.0;
// 2 Pane 分割中にフォーカスがある文書を示す面
const FOCUSED_PANE_SURFACE: ModelSurfaceStyle =
    ModelSurfaceStyle::ThemeBackgroundHighlight { alpha: 0.5 };
impl WorldLayout {
    pub fn next(&self) -> Self {
        match self {
            WorldLayout::Liner => WorldLayout::Circle,
            WorldLayout::Circle => WorldLayout::Liner,
            WorldLayout::Split(_) => WorldLayout::Liner,
        }
    }

    fn layout(&self, world: &mut DefaultWorld) {
        match self {
            WorldLayout::Split(direction) => {
                let Some([first, second]) = world.split_panes else {
                    WorldLayout::Liner.layout(world);
                    return;
                };
                let (w1, h1) = world.models[first].bound();
                let (w2, h2) = world.models[second].bound();
                let (mut position1, mut position2) = match direction {
                    Direction::Vertical => (
                        Vec3::new(-(w2 + INTERVAL) / 2.0, 0.0, 0.0),
                        Vec3::new((w1 + INTERVAL) / 2.0, 0.0, 0.0),
                    ),
                    Direction::Horizontal => (
                        Vec3::new(0.0, (h2 + INTERVAL) / 2.0, 0.0),
                        Vec3::new(0.0, -(h1 + INTERVAL) / 2.0, 0.0),
                    ),
                };
                // 全体表示（reset-zoom）では揃えると倍率が変わり続けるため、従来どおり中心の高さを揃えて並べる
                if world.aligns_split_carets() {
                    // フォーカス側の文書は現在の高さを保ち、もう一方の文書は後でカーソルを画面の中心に合わせる
                    let anchor_y = world.models[world.focus].last_position().y;
                    (position1.y, position2.y) = (anchor_y, anchor_y);
                }
                for (index, position) in [(first, position1), (second, position2)] {
                    world.models[index].set_rotation(Quat::IDENTITY);
                    world.models[index].set_position(position);
                }
                world.center_unfocused_pane_caret();
                let focus_position = world
                    .models
                    .get(world.focus)
                    .map(|model| model.focus_position())
                    .unwrap_or(Vec3::ZERO);
                for modal in world.modal_models.iter_mut() {
                    modal.model.set_position(focus_position);
                }
            }
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
                for modal in world.modal_models.iter_mut() {
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
    use crate::easing_value::EasingPointN;

    fn split_test_world() -> DefaultWorld {
        split_test_world_with(&[(12.0, 8.0), (30.0, 5.0), (6.0, 20.0)])
    }

    fn split_test_world_with(sizes: &[(f32, f32)]) -> DefaultWorld {
        let mut world = DefaultWorld::new(WindowSize::new(800, 600));
        for (index, &(width, height)) in sizes.iter().enumerate() {
            let config = crate::ui_context::TextContext {
                min_bound: (width, height).into(),
                ..Default::default()
            };
            let mut model = crate::ui::TextEdit::new(config);
            model.editor_operation(&EditorOperation::InsertString(format!("document {index}")));
            model.set_rotation(Quat::from_rotation_y(0.5));
            world.add(Box::new(model));
        }
        world
    }

    fn assert_split_fits(world: &DefaultWorld) {
        let (target, eye) = world.camera.target_and_eye();
        let camera = Camera::new(
            eye.into(),
            target.into(),
            [0.0, 1.0, 0.0].into(),
            world.camera.aspect(),
            45.0,
            0.1,
            1000.0,
        );
        let matrix = camera.build_view_projection_matrix();
        let mut extent = 0.0_f32;
        for index in world.split_panes.unwrap() {
            let model = &world.models[index];
            let (w, h) = model.bound();
            for x in [-w / 2.0, w / 2.0] {
                for y in [-h / 2.0, h / 2.0] {
                    let clip = matrix * (model.last_position() + Vec3::new(x, y, 0.0)).extend(1.0);
                    assert!(clip.w > 0.0);
                    let projected = clip.truncate() / clip.w;
                    assert!(projected.x.abs() <= 0.901, "{projected:?}");
                    assert!(projected.y.abs() <= 0.901, "{projected:?}");
                    extent = extent.max(projected.x.abs()).max(projected.y.abs());
                }
            }
        }
        assert!((extent - 0.9).abs() < 0.001, "extent: {extent}");
    }

    #[test]
    fn split_layout_fits_both_documents_in_either_direction_and_aspect() {
        for direction in [Direction::Vertical, Direction::Horizontal] {
            for window_size in [WindowSize::new(1200, 600), WindowSize::new(600, 1200)] {
                let mut world = split_test_world();
                world.direction = Direction::Vertical;
                world.split(direction, 2).unwrap();
                world.change_window_size(window_size);
                assert!(world.world_updated);
                world.look_current(CameraAdjustment::FitBoth);
                assert_split_fits(&world);

                let first = &world.models[0];
                let second = &world.models[2];
                assert_eq!(first.rotation(), Quat::IDENTITY);
                assert_eq!(second.rotation(), Quat::IDENTITY);
                match direction {
                    Direction::Vertical => {
                        let gap = second.last_position().x
                            - second.bound().0 / 2.0
                            - (first.last_position().x + first.bound().0 / 2.0);
                        assert!((gap - INTERVAL).abs() < 0.001);
                        assert_eq!(first.last_position().y, second.last_position().y);
                    }
                    Direction::Horizontal => {
                        let gap = first.last_position().y
                            - first.bound().1 / 2.0
                            - (second.last_position().y + second.bound().1 / 2.0);
                        assert!((gap - INTERVAL).abs() < 0.001);
                        assert_eq!(first.last_position().x, second.last_position().x);
                    }
                }
                assert_eq!(world.render_batches().len(), 2);
            }
        }
    }

    #[test]
    fn split_focus_switch_keeps_positions_and_edits_only_the_active_document() {
        let mut world = split_test_world();
        world.split(Direction::Vertical, 2).unwrap();
        let positions: Vec<_> = world
            .models
            .iter()
            .map(|model| model.last_position())
            .collect();
        let camera = world.camera.target_and_eye();

        world.look_other_pane();
        assert_eq!(world.focus, 2);
        world.editor_operation(&EditorOperation::InsertString(" active".into()));
        assert_eq!(world.models[0].to_string(), "document 0");
        assert_eq!(world.models[2].to_string(), "document 2 active");
        world.look_other_pane();
        assert_eq!(world.focus, 0);
        assert_eq!(world.camera.target_and_eye(), camera);
        assert_eq!(
            world
                .models
                .iter()
                .map(|model| model.last_position())
                .collect::<Vec<_>>(),
            positions,
        );

        world.world_updated = false;
        world.camera_operation(CameraOperation::Backward);
        let zoomed_camera = world.camera.target_and_eye();
        world.look_other_pane();
        assert_eq!(world.camera.target_and_eye(), zoomed_camera);
        world.look_other_pane();

        world.look_next(CameraAdjustment::NoCare);
        assert_eq!(world.focus, 1);
        assert_eq!(world.split_panes, Some([1, 2]));
        world.look_next(CameraAdjustment::NoCare);
        assert_eq!(world.focus, 2);
        assert_eq!(world.split_panes, Some([1, 2]));
        world.look_next(CameraAdjustment::NoCare);
        assert_eq!(world.focus, 0);
        assert_eq!(world.split_panes, Some([1, 0]));
        world.look_prev(CameraAdjustment::NoCare);
        assert_eq!(world.focus, 2);
        assert_eq!(world.split_panes, Some([1, 2]));
    }

    #[test]
    fn split_preserves_selected_documents_when_inserting_swapping_and_removing() {
        let mut world = split_test_world();
        world.split(Direction::Horizontal, 2).unwrap();
        world.add_next(Box::new(crate::ui::TextEdit::default()));
        assert_eq!(world.split_panes, Some([0, 3]));
        world.swap_next();
        assert_eq!(world.focus, 1);
        assert_eq!(world.split_panes, Some([1, 3]));
        assert_eq!(world.current_string(), "document 0");
        world.swap_prev();
        assert_eq!(world.focus, 0);
        assert_eq!(world.split_panes, Some([0, 3]));
        assert_eq!(world.current_string(), "document 0");

        world.look_other_pane();
        world.remove_current();
        world.re_layout();
        world.look_current(CameraAdjustment::FitBoth);
        let panes = world.split_panes.unwrap();
        assert!(panes.contains(&0));
        assert!(panes.contains(&world.focus));
        assert_ne!(panes[0], panes[1]);
        assert_split_fits(&world);

        world.remove_current();
        world.re_layout();
        world.look_current(CameraAdjustment::FitBoth);
        assert_split_fits(&world);
        world.remove_current();
        assert_eq!(world.layout, WorldLayout::Liner);
        assert_eq!(world.split_panes, None);
        assert_eq!(world.focus, 0);
    }

    #[test]
    fn split_highlights_only_the_focused_pane_and_restores_original_surfaces() {
        let mut world = split_test_world();
        let focused_surface = ModelSurfaceStyle::Color([0.1, 0.2, 0.3, 0.4]);
        let other_surface = ModelSurfaceStyle::ThemeBackground { alpha: 0.3 };
        world.models[0].set_surface_style(focused_surface);
        world.models[1].set_surface_style(other_surface);
        world.sync_focused_pane_surface();
        assert_eq!(world.models[0].surface_style(), focused_surface);

        world.split(Direction::Vertical, 2).unwrap();
        world.sync_focused_pane_surface();
        assert_eq!(world.models[0].surface_style(), FOCUSED_PANE_SURFACE);
        assert_eq!(world.models[1].surface_style(), other_surface);
        assert_eq!(world.models[2].surface_style(), ModelSurfaceStyle::None);

        world.look_other_pane();
        world.sync_focused_pane_surface();
        assert_eq!(world.models[0].surface_style(), focused_surface);
        assert_eq!(world.models[2].surface_style(), FOCUSED_PANE_SURFACE);

        // 並べ替え後も強調前の面の設定を元のモデルに戻す
        world.look_other_pane();
        world.sync_focused_pane_surface();
        world.swap_next();
        world.sync_focused_pane_surface();
        assert_eq!(world.models[1].surface_style(), FOCUSED_PANE_SURFACE);
        assert_eq!(world.models[0].surface_style(), other_surface);
        world.change_layout(WorldLayout::Liner);
        world.sync_focused_pane_surface();
        assert_eq!(world.models[1].surface_style(), focused_surface);
        assert_eq!(world.models[0].surface_style(), other_surface);
        assert!(
            world
                .models
                .iter()
                .all(|model| model.surface_style() != FOCUSED_PANE_SURFACE)
        );

        // 閉じた文書には強調前の面の設定が戻る
        world.split(Direction::Horizontal, 2).unwrap();
        world.sync_focused_pane_surface();
        assert_eq!(world.models[1].surface_style(), FOCUSED_PANE_SURFACE);
        world.remove_current();
        assert_eq!(world.focused_pane_highlight, None);
        assert_eq!(
            world
                .departing_normal_models()
                .last()
                .unwrap()
                .model
                .surface_style(),
            focused_surface
        );
        world.sync_focused_pane_surface();
        assert_eq!(
            world.models[world.focus].surface_style(),
            FOCUSED_PANE_SURFACE
        );
    }

    #[test]
    fn split_refits_with_the_last_selected_adjustment() {
        let mut world = split_test_world();
        world.split(Direction::Horizontal, 2).unwrap();
        let fit_both_camera = world.camera.target_and_eye();

        world.look_current(CameraAdjustment::FitWidth);
        let fit_width_camera = world.camera.target_and_eye();
        assert_ne!(fit_width_camera, fit_both_camera);

        // カーソル移動などの更新後の再フィットでも直近のフィット方法を使う
        world.world_updated = true;
        world.look_current(CameraAdjustment::NoCare);
        assert_eq!(world.camera.target_and_eye(), fit_width_camera);
        world.look_other_pane();
        world.look_other_pane();
        world.world_updated = true;
        world.look_current(CameraAdjustment::NoCare);
        assert_eq!(world.camera.target_and_eye(), fit_width_camera);

        world.look_current(CameraAdjustment::FitBoth);
        world.world_updated = true;
        world.look_current(CameraAdjustment::NoCare);
        assert_eq!(world.camera.target_and_eye(), fit_both_camera);
        assert_split_fits(&world);
    }

    #[test]
    fn vertical_split_centers_unfocused_caret_only_when_not_fitting_both() {
        let mut world = split_test_world();
        world.models[2].editor_operation(&EditorOperation::InsertString("\nline\nline".into()));
        world.split(Direction::Vertical, 2).unwrap();
        let unfocused_caret_gap = |world: &DefaultWorld, other: usize| {
            world.models[other].focus_position().y - world.camera.target_and_eye().0[1]
        };
        let center_y_gap = |world: &DefaultWorld| {
            world.models[0].last_position().y - world.models[2].last_position().y
        };
        assert!(center_y_gap(&world).abs() < 0.001);
        assert!(unfocused_caret_gap(&world, 2).abs() > 0.001);

        world.look_current(CameraAdjustment::FitHeight);
        assert!(unfocused_caret_gap(&world, 2).abs() < 0.001);
        let next_focused_y = world.models[2].last_position().y;
        world.look_other_pane();
        assert_eq!(world.models[2].last_position().y, next_focused_y);
        assert!(unfocused_caret_gap(&world, 0).abs() < 0.001);

        world.look_current(CameraAdjustment::FitBoth);
        assert!(center_y_gap(&world).abs() < 0.001);
    }

    #[test]
    fn aligned_split_keeps_document_edges_visible_near_the_caret() {
        // GPU なしではカーソルが動かないため、縦長の文書の先頭にカーソルがある状態を検証する
        let mut world = split_test_world_with(&[(12.0, 120.0), (12.0, 10.0)]);
        world.split(Direction::Vertical, 1).unwrap();
        world.look_current(CameraAdjustment::FitWidth);

        let (target, eye) = world.camera.target_and_eye();
        let (target, eye) = (Vec3::from(target), Vec3::from(eye));
        let visible_half = (eye - target).length() * (45.0_f32.to_radians() / 2.0).tan();
        let model = &world.models[0];
        let top = model.last_position().y + model.bound().1 / 2.0;
        assert!(visible_half < model.bound().1 / 2.0);
        // 文書の上端が画面内に残り、カーソルは画面中央に来ない
        assert!((top - target.y).abs() <= visible_half + 0.001);
        assert!((model.focus_position().y - target.y).abs() > 1.0);
        // フォーカスの無い文書のカーソルは画面の中心に来る
        assert!((world.models[1].focus_position().y - target.y).abs() < 0.001);
    }

    #[test]
    fn split_rejects_invalid_selections_and_preserves_modal_focus() {
        let mut world = split_test_world();
        assert!(matches!(
            world.split(Direction::Vertical, 0),
            Err(SplitError::SameModel)
        ));
        assert!(matches!(
            world.split(Direction::Vertical, 99),
            Err(SplitError::InvalidModel(99))
        ));
        assert_eq!(world.layout, WorldLayout::Liner);
        world.split(Direction::Vertical, 2).unwrap();
        world.add_modal(Box::new(crate::ui::TextEdit::default()));
        assert!(matches!(
            world.split(Direction::Horizontal, 1),
            Err(SplitError::ModalOpen)
        ));
        world.look_other_pane();
        assert_eq!(world.focus, 0);
        world.remove_current();
        world.look_other_pane();
        assert_eq!(world.focus, 2);
        world.change_layout(WorldLayout::Liner);
        assert_eq!(world.split_panes, None);
        assert_eq!(world.visible_models().count(), 3);
    }

    #[test]
    fn switch_model_replaces_or_swaps_the_focused_pane() {
        let mut world = split_test_world();
        world.switch_model(1).unwrap();
        assert_eq!((world.focus, world.split_panes), (1, None));

        world.split(Direction::Vertical, 0).unwrap();
        world.switch_model(2).unwrap();
        assert_eq!((world.focus, world.split_panes), (2, Some([2, 0])));
        assert_split_fits(&world);

        world.switch_model(0).unwrap();
        assert_eq!((world.focus, world.split_panes), (0, Some([0, 2])));
        assert_split_fits(&world);

        assert!(matches!(
            world.switch_model(99),
            Err(SwitchModelError::InvalidModel(99))
        ));
        world.add_modal(Box::new(crate::ui::TextEdit::default()));
        assert!(matches!(
            world.switch_model(2),
            Err(SwitchModelError::ModalOpen)
        ));
        assert_eq!(world.split_panes, Some([0, 2]));
    }

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
        assert_eq!(world.modal_models.len(), 1);
        let departing = world.departing_models.iter().next().unwrap();
        assert_eq!(departing.layer, DepartingLayer::Modal { order: 1 });
        assert_eq!(departing.model.last_position(), last_position);
        assert_eq!(departing.fade.opacity.last(), [0.0]);
        assert_eq!(world.render_batches().len(), 3);
        assert!(matches!(world.remove_current(), RemovedModelType::Modal));
        assert!(!world.has_modal());
        assert_eq!(world.departing_normal_models().count(), 0);
        assert!(world.modal_instances().0.is_empty());
        assert_eq!(
            world.get_current_mut().unwrap().last_position(),
            Vec3::new(1.0, 2.0, 3.0)
        );
        assert_eq!(world.render_batches().len(), 3);
        // 破棄時刻を過ぎると退場中のモーダルは描画されなくなる
        world.departing_models.sweep(u32::MAX, |_, _| {});
        assert_eq!(world.render_batches().len(), 1);
    }

    #[test]
    fn replace_models_retires_visible_models_and_resets_split() {
        let mut world = DefaultWorld::new(WindowSize::new(800, 600));
        for _ in 0..20 {
            world.add(Box::new(crate::ui::TextEdit::default()));
        }
        world.split(Direction::Horizontal, 1).unwrap();
        world.add_modal(Box::new(crate::ui::TextEdit::default()));

        world.replace_models([
            Box::new(crate::ui::TextEdit::default()) as Box<dyn Model>,
            Box::new(crate::ui::TextEdit::default()),
        ]);

        assert_eq!(world.model_length(), 2);
        assert_eq!(world.focus, 0);
        assert_eq!(world.split_panes, None);
        assert_eq!(world.layout, WorldLayout::Liner);
        assert!(!world.has_modal());
        // 分割中に見えていた 2 つのペインとモーダルだけが退場アニメーションを行う
        assert_eq!(world.departing_normal_models().count(), 2);
        assert_eq!(world.departing_models.len(), 3);
    }

    #[test]
    fn removed_normal_model_fades_out_below_its_position() {
        let mut world = DefaultWorld::new(WindowSize::new(800, 600));
        world.add(Box::new(crate::ui::TextEdit::default()));
        world.add(Box::new(crate::ui::TextEdit::default()));
        let position = world.models[0].position();

        assert!(matches!(world.remove_current(), RemovedModelType::Normal));
        assert_eq!(world.models.len(), 1);
        let departing = world.departing_normal_models().next().unwrap();
        assert_eq!(
            departing.model.last_position(),
            position - Vec3::Y * MODEL_EXIT_SLIDE
        );
        assert_eq!(departing.fade.opacity.last(), [0.0]);
        assert_eq!(world.render_batches().len(), 2);

        world.departing_models.sweep(u32::MAX, |_, _| {});
        assert_eq!(world.render_batches().len(), 1);
    }

    #[test]
    fn closing_modals_keep_their_layer_when_another_modal_opens() {
        let mut world = DefaultWorld::new(WindowSize::new(800, 600));
        world.add(Box::new(crate::ui::TextEdit::default()));
        world.add_modal(Box::new(crate::ui::TextEdit::default()));
        world.add_modal(Box::new(crate::ui::TextEdit::default()));
        assert!(matches!(world.remove_current(), RemovedModelType::Modal));

        for (_, departing) in world.departing_models.iter_mut() {
            departing.fade.opacity = EasingPointN::new([0.5]);
        }
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
