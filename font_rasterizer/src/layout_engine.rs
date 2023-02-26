use cgmath::{Point3, Quaternion};

use crate::camera::Camera;

pub trait World {
    fn add(&mut self, model: Box<dyn Model>);
    fn re_layout(&mut self);
    fn model_length(&self) -> usize;
    fn look_at(&mut self, model_num: usize);
}

pub struct HorizontalWorld {
    camera: Camera,
    models: Vec<Box<dyn Model>>,
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
        let Some(_model) = self.models.get(model_index) else {return};
    }
}

pub trait Model {
    fn set_position(&mut self, position: Point3<f32>);
    fn position(&self) -> Point3<f32>;
    fn rotation(&self) -> Quaternion<f32>;
    fn length(&self) -> f32;
}
