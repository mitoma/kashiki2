use std::{
    collections::{BTreeMap, HashSet},
    ops::Range,
    sync::Arc,
};

use cgmath::{Matrix4, Point2, Point3, Quaternion, Rotation3};
use log::info;
use text_buffer::{action::EditorOperation, editor::CharWidthResolver};

use font_rasterizer::{
    context::{StateContext, WindowSize},
    glyph_instances::GlyphInstances,
    glyph_vertex_buffer::Direction,
    vector_instances::VectorInstances,
};

use crate::camera::{Camera, CameraAdjustment, CameraController, CameraOperation};

// 画面全体を表す
pub trait World {
    // model を追加する
    fn add(&mut self, model: Box<dyn Model>);
    // model を現在のモデルの次に追加する
    fn add_next(&mut self, model: Box<dyn Model>);
    // 現在参照している model を削除する
    fn remove_current(&mut self);

    // 再レイアウトする update するときに呼び出すとよさそう
    fn re_layout(&mut self);

    fn update(&mut self, context: &StateContext);

    // この World にいくつモデルを配置されているかを返す
    fn model_length(&self) -> usize;
    // 何番目のモデルに視点を移すか
    fn look_at(&mut self, model_num: usize, adjustment: CameraAdjustment);

    // 現在のモデルに再度視点を移す
    fn look_current(&mut self, adjustment: CameraAdjustment);
    // 次のモデルに視点を移す
    fn look_next(&mut self, adjustment: CameraAdjustment);
    // 前のモデルに視点を移す
    fn look_prev(&mut self, adjustment: CameraAdjustment);
    // 次のモデルに視点を移す
    fn swap_next(&mut self);
    // 次のモデルに視点を移す
    fn swap_prev(&mut self);
    // カメラの参照を返す
    fn camera(&self) -> &Camera;
    // カメラを動かす
    fn camera_operation(&mut self, camera_operation: CameraOperation);
    // ウィンドウサイズ変更の通知を受け取る
    fn change_window_size(&mut self, window_size: WindowSize);
    // レイアウトを変更する
    fn change_layout(&mut self, layout: WorldLayout);
    // レイアウトを返す
    fn layout(&self) -> &WorldLayout;
    // glyph_instances を返す
    fn glyph_instances(&self) -> Vec<&GlyphInstances>;
    // vector_instances を返す
    fn vector_instances(&self) -> Vec<&VectorInstances<String>>;

    fn editor_operation(&mut self, op: &EditorOperation);
    fn model_operation(&mut self, op: &ModelOperation);
    fn current_string(&self) -> String;
    fn strings(&self) -> Vec<String>;
    fn chars(&self) -> HashSet<char>;

    // 今フォーカスが当たっているモデルのモードを返す
    fn current_model_mode(&self) -> Option<ModelMode>;

    // カメラの位置を変更する。x_ratio, y_ratio はそれぞれ -1.0 から 1.0 までの値をとり、
    // アプリケーションのウインドウ上の位置を表す。(0.0, 0.0) はウインドウの中心を表す。
    fn move_to_position(&mut self, x_ratio: f32, y_ratio: f32);
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

                    let rotation = cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_y(),
                        cgmath::Deg(0.0),
                    );
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

                            let rotation = cgmath::Quaternion::from_axis_angle(
                                cgmath::Vector3::unit_y(),
                                cgmath::Deg(-r.to_degrees()),
                            );
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

                            let rotation = cgmath::Quaternion::from_axis_angle(
                                cgmath::Vector3::unit_x(),
                                cgmath::Deg(-r.to_degrees()),
                            );
                            model.set_rotation(rotation);
                        }
                    }
                }
            }
        }
    }
}

pub struct DefaultWorld {
    camera: Camera,
    camera_controller: CameraController,
    models: Vec<Box<dyn Model>>,
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
            camera_controller: CameraController::new(5.0),
            models: Vec::new(),
            removed_models: Vec::new(),
            focus: 0,
            world_updated: true,
            direction: Direction::Horizontal,
            layout: WorldLayout::Liner,
        }
    }

    fn get_current_mut(&mut self) -> Option<&mut Box<dyn Model>> {
        self.models.get_mut(self.focus)
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

    fn camera(&self) -> &Camera {
        &self.camera
    }

    fn glyph_instances(&self) -> Vec<&GlyphInstances> {
        let models: Vec<&GlyphInstances> = self.models[self.get_surrounding_model_range()]
            .iter()
            .flat_map(|m| m.glyph_instances())
            .collect();
        let removed_models: Vec<&GlyphInstances> = self
            .removed_models
            .iter()
            .flat_map(|m| m.glyph_instances())
            .collect();
        models
            .iter()
            .chain(removed_models.iter())
            .cloned()
            .collect()
    }

    fn vector_instances(&self) -> Vec<&VectorInstances<String>> {
        let models: Vec<&VectorInstances<String>> = self.models[self.get_surrounding_model_range()]
            .iter()
            .flat_map(|m| m.vector_instances())
            .collect();
        let removed_models: Vec<&VectorInstances<String>> = self
            .removed_models
            .iter()
            .flat_map(|m| m.vector_instances())
            .collect();
        models
            .iter()
            .chain(removed_models.iter())
            .cloned()
            .collect()
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
        self.look_at(self.focus, adjustment)
    }

    fn look_next(&mut self, adjustment: CameraAdjustment) {
        // model がない場合は何もしない
        if self.model_length() == 0 {
            return;
        }

        // modal の場合はフォーカスを移動させない
        if let Some(ModelMode::Modal) = self.current_model_mode() {
            self.look_current(adjustment);
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
        if let Some(ModelMode::Modal) = self.current_model_mode() {
            self.look_current(adjustment);
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
        self.models
            .iter()
            // modal は保存対象の文字列ではない
            .filter(|m| m.model_mode() != ModelMode::Modal)
            .map(|m| m.to_string())
            .collect()
    }

    fn remove_current(&mut self) {
        self.world_updated = true;
        let mut removed_model = self.models.remove(self.focus);
        let (x, y, z) = removed_model.position().into();
        removed_model.set_position(Point3::new(x, y - 5.0, z));
        self.removed_models.push(removed_model);
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

    fn current_model_mode(&self) -> Option<ModelMode> {
        self.models.get(self.focus).map(|m| m.model_mode())
    }

    fn move_to_position(&mut self, x_ratio: f32, y_ratio: f32) {
        let mut distance_map: BTreeMap<usize, f32> = BTreeMap::new();

        for (idx, model) in self.models.iter().enumerate() {
            let Point3 { x, y, z } = model.position();
            let position_vec = cgmath::Vector3 { x, y, z };

            let p = cgmath::Matrix4::from_translation(position_vec)
                * cgmath::Matrix4::from(model.rotation());
            let view_projection_matrix = self.camera.build_view_projection_matrix();
            let calced_model_position = view_projection_matrix * p;
            let nw = calced_model_position.w;
            let nw_x = nw.x / nw.w;
            let nw_y = nw.y / nw.w;
            let distance = (x_ratio - nw_x).abs().powf(2.0) + (y_ratio - nw_y).abs().powf(2.0);
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
pub enum ModelMode {
    Nomal,
    Modal,
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum ModelBorder {
    #[default]
    None,
    Square,
    Rounded,
}

pub trait Model {
    // モデルの位置を設定する
    fn set_position(&mut self, position: Point3<f32>);
    // モデルの位置を返す
    fn position(&self) -> Point3<f32>;
    // モデル中、カメラがフォーカスすべき位置を返す
    // position はモデルの中心を指す
    fn focus_position(&self) -> Point3<f32>;
    // モデルの回転を設定する
    fn set_rotation(&mut self, rotation: Quaternion<f32>);
    // モデルの回転を返す
    fn rotation(&self) -> Quaternion<f32>;
    // モデルの縦横の長さを返す
    fn bound(&self) -> (f32, f32);
    fn glyph_instances(&self) -> Vec<&GlyphInstances>;
    fn vector_instances(&self) -> Vec<&VectorInstances<String>>;
    fn update(&mut self, context: &StateContext);
    fn editor_operation(&mut self, op: &EditorOperation);
    fn model_operation(&mut self, op: &ModelOperation) -> ModelOperationResult;
    fn to_string(&self) -> String;
    fn model_mode(&self) -> ModelMode;
    fn in_animation(&self) -> bool;
    fn set_border(&mut self, border: ModelBorder);
    fn border(&self) -> ModelBorder;
}

pub enum ModelOperation {
    ChangeDirection(Option<Direction>),
    // 行間を増加させる
    IncreaseRowInterval,
    // 行間を減少させる
    DecreaseRowInterval,
    // 行間の大きさを増加させる
    IncreaseRowScale,
    // 行間の大きさを減少させる
    DecreaseRowScale,
    // 文字間を増加させる
    IncreaseColInterval,
    // 文字間を減少させる
    DecreaseColInterval,
    // 文字間の大きさを増加させる
    IncreaseColScale,
    // 文字間の大きさを減少させる
    DecreaseColScale,
    // 文章の最小サイズを切り替える
    ToggleMinBound,
    // Copy Display String
    CopyDisplayString(Arc<dyn CharWidthResolver>, fn(String)),
    // サイケデリックモードを切り替える(実験的なお遊び機能)
    TogglePsychedelic,
    // Click
    MoveToClick(f32, f32, Matrix4<f32>),
    MarkAndClick(f32, f32, Matrix4<f32>),
}

pub enum ModelOperationResult {
    NoCare,
    RequireReLayout,
}

// モデルが持つ属性をまとめたもの
pub struct ModelAttributes {
    pub center: Point2<f32>,
    // モデルの world 空間上の位置
    pub position: Point3<f32>,
    // モデルの world 空間上の回転(向き)
    pub rotation: Quaternion<f32>,
    // モデルの world 空間上の拡大率(x, y)
    pub world_scale: [f32; 2],
}
