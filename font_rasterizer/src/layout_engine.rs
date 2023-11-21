use cgmath::{Point3, Quaternion};

use crate::{
    camera::{Camera, CameraController},
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
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );

    // この World にいくつモデルを配置されているかを返す
    fn model_length(&self) -> usize;
    // 何番目のモデルに視点を移すか
    fn look_at(&mut self, model_num: usize);
    // カメラの参照を返す
    fn camera(&self) -> &Camera;
    // ウィンドウサイズ変更の通知を受け取る
    fn change_window_size(&mut self, window_size: (u32, u32));
    // glyph_instances を返す
    fn glyph_instances(&self) -> Vec<&GlyphInstances>;
}

pub struct HorizontalWorld {
    camera: Camera,
    camera_controller: CameraController,
    models: Vec<Box<dyn Model>>,
}

impl HorizontalWorld {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            camera: Camera::basic((width, height)),
            camera_controller: CameraController::new(10.0),
            models: Vec::new(),
        }
    }
}

impl World for HorizontalWorld {
    fn add(&mut self, model: Box<dyn Model>) {
        self.models.push(model);
    }

    fn re_layout(&mut self) {
        let mut y_position = 0.0;
        for model in self.models.iter_mut() {
            model.set_position((y_position, 0.0, 0.0).into());
            y_position += model.length();
        }
    }

    fn model_length(&self) -> usize {
        self.models.len()
    }

    fn look_at(&mut self, model_index: usize) {
        let Some(model) = self.models.get(model_index) else {
            return;
        };
        self.camera_controller
            .look_at(&mut self.camera, model.as_ref());
        self.camera_controller
            .process(&crate::camera::CameraOperation::Backward);
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
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        for model in self.models.iter_mut() {
            model.update(glyph_vertex_buffer, device, queue);
        }
    }

    fn change_window_size(&mut self, window_size: (u32, u32)) {
        self.camera_controller
            .update_camera_aspect(&mut self.camera, window_size.0, window_size.1);
    }
}

pub trait Model {
    fn set_position(&mut self, position: Point3<f32>);
    fn position(&self) -> Point3<f32>;
    fn rotation(&self) -> Quaternion<f32>;
    fn length(&self) -> f32;
    fn glyph_instances(&self) -> Vec<&GlyphInstances>;
    fn update(
        &mut self,
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );
}
