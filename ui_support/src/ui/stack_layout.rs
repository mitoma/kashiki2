use font_rasterizer::glyph_vertex_buffer::Direction;
use glam::Quat;
use rand::rand_core::le;

use crate::{easing_value::EasingPointN, layout_engine::Model};

/// Model を複数まとめてレイアウトするためのコンテナ
/// Direction に応じて子モデルを縦または横に並べる
pub struct StackLayout {
    direction: Direction,

    models: Vec<Box<dyn Model>>,
    focus_model_index: Option<usize>,

    position: EasingPointN<3>,
    rotation: EasingPointN<4>,
    bound: EasingPointN<2>,
}

impl StackLayout {
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            models: Vec::new(),
            focus_model_index: None,
            position: EasingPointN::new([0.0, 0.0, 0.0]),
            rotation: EasingPointN::new([0.0, 0.0, 0.0, 1.0]),
            bound: EasingPointN::new([0.0, 0.0]),
        }
    }

    pub fn add_model(&mut self, model: Box<dyn Model>) {
        self.models.push(model);
    }

    pub fn set_focus_model_index(&mut self, index: usize) {
        self.focus_model_index = Some(index);
    }
}

impl Model for StackLayout {
    fn set_position(&mut self, position: glam::Vec3) {
        self.position.update(position.into());
    }

    fn position(&self) -> glam::Vec3 {
        self.position.current().into()
    }

    fn last_position(&self) -> glam::Vec3 {
        self.position.last().into()
    }

    fn focus_position(&self) -> glam::Vec3 {
        self.position.current().into()
    }

    fn set_rotation(&mut self, rotation: glam::Quat) {
        self.rotation.update(rotation.into());
    }

    fn rotation(&self) -> glam::Quat {
        Quat::from_array(self.rotation.current().into())
    }

    fn bound(&self) -> (f32, f32) {
        let total_width = self.models.iter().map(|model| model.bound().0).sum::<f32>();
        let total_height = self.models.iter().map(|model| model.bound().1).sum::<f32>();
        match self.direction {
            Direction::Horizontal => (
                total_width,
                self.models.iter().map(|m| m.bound().1).fold(0.0, f32::max),
            ),
            Direction::Vertical => (
                self.models.iter().map(|m| m.bound().0).fold(0.0, f32::max),
                total_height,
            ),
        }
    }

    fn glyph_instances(&self) -> Vec<&font_rasterizer::glyph_instances::GlyphInstances> {
        self.models
            .iter()
            .flat_map(|model| model.glyph_instances())
            .collect()
    }

    fn vector_instances(&self) -> Vec<&font_rasterizer::vector_instances::VectorInstances<String>> {
        self.models
            .iter()
            .flat_map(|model| model.vector_instances())
            .collect()
    }

    fn update(&mut self, context: &crate::ui_context::UiContext) {
        let position: glam::Vec3 = self.position.current().into();
        match self.direction {
            Direction::Horizontal => {
                let mut current_y = position.y;
                for model in self.models.iter_mut() {
                    let bound = model.bound();
                    model.set_position(glam::vec3(
                        position.x,
                        current_y - bound.1 / 2.0,
                        position.z,
                    ));
                    current_y -= bound.1;
                }
            }
            Direction::Vertical => {
                let mut current_x = position.x;
                for model in self.models.iter_mut() {
                    let bound = model.bound();
                    model.set_position(glam::vec3(
                        current_x + bound.0 / 2.0,
                        position.y,
                        position.z,
                    ));
                    current_x += bound.0;
                }
            }
        }

        for model in self.models.iter_mut() {
            model.update(context);
        }
    }

    fn editor_operation(&mut self, op: &text_buffer::action::EditorOperation) {
        if let Some(index) = self.focus_model_index {
            if let Some(model) = self.models.get_mut(index) {
                model.editor_operation(op);
            }
        }
    }

    fn model_operation(
        &mut self,
        op: &crate::layout_engine::ModelOperation,
    ) -> crate::layout_engine::ModelOperationResult {
        if let Some(index) = self.focus_model_index {
            if let Some(model) = self.models.get_mut(index) {
                return model.model_operation(op);
            }
        }
        crate::layout_engine::ModelOperationResult::NoCare
    }

    fn to_string(&self) -> String {
        self.models
            .iter()
            .map(|model| model.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn in_animation(&self) -> bool {
        self.models.iter().any(|model| model.in_animation())
    }

    fn set_border(&mut self, border: crate::layout_engine::ModelBorder) {
        for model in self.models.iter_mut() {
            model.set_border(border.clone());
        }
    }

    fn border(&self) -> crate::layout_engine::ModelBorder {
        crate::layout_engine::ModelBorder::None
    }

    fn set_easing_preset(&mut self, preset: crate::ui_context::CharEasingsPreset) {
        for model in self.models.iter_mut() {
            model.set_easing_preset(preset.clone());
        }
    }
}
