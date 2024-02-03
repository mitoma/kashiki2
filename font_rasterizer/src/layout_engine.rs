use cgmath::{Point3, Quaternion};
use log::info;
use text_buffer::action::EditorOperation;

use crate::{
    camera::{Camera, CameraAdjustment, CameraController, CameraOperation},
    color_theme::ColorTheme,
    font_buffer::GlyphVertexBuffer,
    instances::GlyphInstances,
};

// 画面全体を表す
pub trait World {
    // model を追加する
    fn add(&mut self, model: Box<dyn Model>);
    // 再レイアウトする update するときに呼び出すとよさそう
    fn re_layout(&mut self);

    fn update(
        &mut self,
        color_theme: &ColorTheme,
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );

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

    // カメラの参照を返す
    fn camera(&self) -> &Camera;
    // カメラを動かす
    fn camera_operation(&mut self, camera_operation: CameraOperation);
    // ウィンドウサイズ変更の通知を受け取る
    fn change_window_size(&mut self, window_size: (u32, u32));
    // glyph_instances を返す
    fn glyph_instances(&self) -> Vec<&GlyphInstances>;

    fn operation(&mut self, op: &EditorOperation);
    fn model_operation(&mut self, op: &ModelOperation);
}

pub struct HorizontalWorld {
    camera: Camera,
    camera_controller: CameraController,
    models: Vec<Box<dyn Model>>,
    focus: usize,
}

impl HorizontalWorld {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            camera: Camera::basic((width, height)),
            camera_controller: CameraController::new(10.0),
            models: Vec::new(),
            focus: 0,
        }
    }

    fn get_current_mut(&mut self) -> Option<&mut Box<dyn Model>> {
        self.models.get_mut(self.focus)
    }
}
const INTERVAL: f32 = 5.0;

impl World for HorizontalWorld {
    fn add(&mut self, model: Box<dyn Model>) {
        self.models.push(model);
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
        self.models
            .iter()
            .flat_map(|m| m.glyph_instances())
            .collect()
    }

    fn update(
        &mut self,
        color_theme: &ColorTheme,
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        for model in self.models.iter_mut() {
            model.update(color_theme, glyph_vertex_buffer, device, queue);
        }
    }

    fn change_window_size(&mut self, window_size: (u32, u32)) {
        self.camera_controller
            .update_camera_aspect(&mut self.camera, window_size.0, window_size.1);
    }

    fn camera_operation(&mut self, camera_operation: CameraOperation) {
        self.camera_controller.process(&camera_operation);
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_controller.reset_state();
    }

    fn operation(&mut self, op: &EditorOperation) {
        self.get_current_mut().map(|model| model.operation(op));
    }

    fn look_current(&mut self, adjustment: CameraAdjustment) {
        self.look_at(self.focus, adjustment)
    }

    fn look_next(&mut self, adjustment: CameraAdjustment) {
        let next = (self.focus + 1) % self.model_length();
        self.look_at(next, adjustment)
    }

    fn look_prev(&mut self, adjustment: CameraAdjustment) {
        let prev = if self.focus == 0 {
            self.model_length() - 1
        } else {
            self.focus - 1
        };
        self.look_at(prev, adjustment)
    }

    fn model_operation(&mut self, op: &ModelOperation) {
        self.get_current_mut()
            .map(|model| model.model_operation(op));
    }
}

pub trait Model {
    // モデルの位置を設定する
    fn set_position(&mut self, position: Point3<f32>);
    // モデルの位置を返す
    fn position(&self) -> Point3<f32>;
    // モデルの回転を設定する
    fn rotation(&self) -> Quaternion<f32>;
    // モデルの縦横の長さを返す
    fn bound(&self) -> (f32, f32);
    fn glyph_instances(&self) -> Vec<&GlyphInstances>;
    fn update(
        &mut self,
        color_theme: &ColorTheme,
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );
    fn operation(&mut self, op: &EditorOperation);
    fn model_operation(&mut self, op: &ModelOperation);
}

pub enum ModelOperation {
    ChangeDirection,
}
