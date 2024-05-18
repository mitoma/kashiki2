use std::{collections::BTreeMap, ops::Range};

use cgmath::{Point2, Point3, Quaternion, Rotation3};
use log::info;
use text_buffer::{action::EditorOperation, editor::CharWidthResolver};

use crate::{
    camera::{Camera, CameraAdjustment, CameraController, CameraOperation},
    context::{StateContext, WindowSize},
    font_buffer::GlyphVertexBuffer,
    instances::GlyphInstances,
};

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

    fn update(&mut self, glyph_vertex_buffer: &mut GlyphVertexBuffer, context: &StateContext);

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
    // glyph_instances を返す
    fn glyph_instances(&self) -> Vec<&GlyphInstances>;

    fn editor_operation(&mut self, op: &EditorOperation);
    fn model_operation(&mut self, op: &ModelOperation);
    fn current_string(&self) -> String;
    fn strings(&self) -> Vec<String>;

    // 今フォーカスが当たっているモデルのモードを返す
    fn current_model_mode(&self) -> Option<ModelMode>;

    // カメラの位置を変更する。x_ratio, y_ratio はそれぞれ -1.0 から 1.0 までの値をとり、
    // アプリケーションのウインドウ上の位置を表す。(0.0, 0.0) はウインドウの中心を表す。
    fn move_to_position(&mut self, x_ratio: f32, y_ratio: f32);
}

pub struct HorizontalWorld {
    camera: Camera,
    camera_controller: CameraController,
    models: Vec<Box<dyn Model>>,
    focus: usize,
    world_updated: bool,
}

impl HorizontalWorld {
    pub fn new(window_size: WindowSize) -> Self {
        Self {
            camera: Camera::basic(window_size),
            camera_controller: CameraController::new(10.0),
            models: Vec::new(),
            focus: 0,
            world_updated: true,
        }
    }

    fn get_current_mut(&mut self) -> Option<&mut Box<dyn Model>> {
        self.models.get_mut(self.focus)
    }

    fn get_surrounding_model_range(&self) -> Range<usize> {
        let around = 5;
        let min = if self.focus > around {
            self.focus - around
        } else {
            0
        };
        let max = if self.focus + around < self.models.len() {
            self.focus + around
        } else {
            self.models.len()
        };
        min..max
    }
}
const INTERVAL: f32 = 5.0;

impl World for HorizontalWorld {
    fn add(&mut self, model: Box<dyn Model>) {
        self.models.push(model);
    }

    fn add_next(&mut self, model: Box<dyn Model>) {
        self.models.insert(self.focus + 1, model);
    }

    fn re_layout(&mut self) {
        let mut x_position = 0.0;
        for (idx, model) in self.models.iter_mut().enumerate() {
            let (w, h) = model.bound();
            info!("w: {}, h: {}, idx:{}", w, h, idx);
            x_position += w / 2.0;
            model.set_position((x_position, 0.0, 0.0).into());
            x_position += w / 2.0;
            x_position += INTERVAL;

            let rotation =
                cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_y(), cgmath::Deg(0.0));
            model.set_rotation(rotation);
        }
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
        self.models[self.get_surrounding_model_range()]
            .iter()
            .flat_map(|m| m.glyph_instances())
            .collect()
    }

    fn update(&mut self, _glyph_vertex_buffer: &mut GlyphVertexBuffer, context: &StateContext) {
        let range = self.get_surrounding_model_range();
        for model in self.models[range].iter_mut() {
            model.update(context);
        }
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
        // modal の場合はフォーカスを移動させない
        if let Some(ModelMode::Modal) = self.current_model_mode() {
            self.look_current(adjustment);
            return;
        }
        let next = (self.focus + 1) % self.model_length();
        self.look_at(next, adjustment)
    }

    fn look_prev(&mut self, adjustment: CameraAdjustment) {
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
        self.models.remove(self.focus);
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

            info!("model: {}, nw_x: {:?}, nw_y: {:?}", idx, nw_x, nw_y);

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
                // TODO モデル中の最も近いモデルにカーソルを移動するという制御を行いたい
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelMode {
    Nomal,
    Modal,
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
    fn update(&mut self, context: &StateContext);
    fn editor_operation(&mut self, op: &EditorOperation);
    fn model_operation(&mut self, op: &ModelOperation) -> ModelOperationResult;
    fn to_string(&self) -> String;
    fn model_mode(&self) -> ModelMode;
}

pub enum ModelOperation<'a> {
    ChangeDirection,
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
    // Copy Display String
    CopyDisplayString(&'a dyn CharWidthResolver, fn(String)),
    // サイケデリックモードを切り替える(実験的なお遊び機能)
    TogglePsychedelic,
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
