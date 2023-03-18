use cgmath::{Point3, Quaternion};

use crate::camera::{Camera, CameraController};

pub trait World {
    fn add(&mut self, model: Box<dyn Model>);
    fn re_layout(&mut self);
    fn model_length(&self) -> usize;
    fn look_at(&mut self, model_num: usize);
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
        let Some(model) = self.models.get(model_index) else {return};
        self.camera_controller.look_at(&mut self.camera, model);
    }
}

pub trait Model {
    fn set_position(&mut self, position: Point3<f32>);
    fn position(&self) -> Point3<f32>;
    fn rotation(&self) -> Quaternion<f32>;
    fn length(&self) -> f32;
}
