use font_rasterizer::glyph_vertex_buffer::Direction;
use glam::Quat;

use crate::{
    easing_value::EasingPointN,
    layout_engine::{Model, ModelOperation, ModelOperationResult},
};

/// Model を複数まとめてレイアウトするためのコンテナ
/// Direction に応じて子モデルを縦または横に並べる
pub struct StackLayout {
    direction: Direction,

    models: Vec<Box<dyn Model>>,
    focus_model_index: Option<usize>,

    position: EasingPointN<3>,
    rotation: EasingPointN<4>,

    margin: Margin,
}

pub struct Margin {
    horizontal: f32,
    vertical: f32,
}

impl StackLayout {
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            models: Vec::new(),
            focus_model_index: None,
            position: EasingPointN::new([0.0, 0.0, 0.0]),
            rotation: EasingPointN::new([0.0, 0.0, 0.0, 1.0]),
            margin: Margin {
                horizontal: 0.1,
                vertical: 0.1,
            },
        }
    }

    pub fn add_model(&mut self, model: Box<dyn Model>) {
        self.models.push(model);
    }

    pub fn set_focus_model_index(&mut self, index: usize) {
        self.focus_model_index = Some(index);
    }

    pub fn set_margin(&mut self, horizontal: f32, vertical: f32) {
        self.margin.horizontal = horizontal;
        self.margin.vertical = vertical;
    }

    pub fn models(&self) -> &Vec<Box<dyn Model>> {
        &self.models
    }

    pub fn models_mut(&mut self) -> &mut Vec<Box<dyn Model>> {
        &mut self.models
    }

    pub fn direction(&self) -> Direction {
        self.direction
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

    // フォーカスモデルがあればそのモデルの focus_position を返す。なければ全体の position を返す
    fn focus_position(&self) -> glam::Vec3 {
        if let Some(index) = self.focus_model_index
            && let Some(model) = self.models.get(index)
        {
            return model.focus_position();
        }
        self.last_position()
    }

    fn set_rotation(&mut self, rotation: glam::Quat) {
        self.rotation.update(rotation.into());
    }

    fn rotation(&self) -> glam::Quat {
        Quat::from_array(self.rotation.current())
    }

    fn bound(&self) -> (f32, f32) {
        match self.direction {
            Direction::Horizontal => (
                // 縦積み: 幅は最大値、高さは合計
                self.models.iter().map(|m| m.bound().0).fold(0.0, f32::max),
                self.models.iter().map(|m| m.bound().1).sum::<f32>()
                    + self.margin.vertical * (self.models.len() as f32 - 1.0),
            ),
            Direction::Vertical => (
                // 横積み: 幅は合計、高さは最大値
                self.models.iter().map(|m| m.bound().0).sum::<f32>()
                    + self.margin.horizontal * (self.models.len() as f32 - 1.0),
                self.models.iter().map(|m| m.bound().1).fold(0.0, f32::max),
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
        let position: glam::Vec3 = self.last_position();
        let layout_bound: (f32, f32) = self.bound();
        log::info!("--------");
        log::info!(
            "StackLayout update: position={:?}, bound={:?}",
            position,
            layout_bound
        );
        match self.direction {
            Direction::Horizontal => {
                let mut current_y = position.y + layout_bound.1 / 2.0;
                for model in self.models.iter_mut() {
                    let bound = model.bound();
                    current_y -= bound.1 / 2.0;
                    log::info!(
                        "model bound: {:?}, model_position: x={}, y={}",
                        bound,
                        position.x,
                        current_y
                    );
                    model.set_position(glam::vec3(position.x, current_y, position.z));
                    current_y -= bound.1 / 2.0;
                    current_y -= self.margin.vertical;
                }
            }
            Direction::Vertical => {
                let mut current_x = position.x + layout_bound.0 / 2.0;
                for model in self.models.iter_mut() {
                    let bound = model.bound();
                    current_x -= (bound.0 + self.margin.horizontal) / 2.0;
                    log::info!(
                        "model bound: {:?}, model_position: x={}, y={}",
                        bound,
                        current_x,
                        position.y
                    );
                    model.set_position(glam::vec3(current_x, position.y, position.z));
                    current_x -= (bound.0 + self.margin.horizontal) / 2.0;
                }
            }
        }

        for model in self.models.iter_mut() {
            model.update(context);
        }
    }

    fn editor_operation(&mut self, op: &text_buffer::action::EditorOperation) {
        if let Some(index) = self.focus_model_index
            && let Some(model) = self.models.get_mut(index)
        {
            model.editor_operation(op);
        }
    }

    fn model_operation(
        &mut self,
        op: &crate::layout_engine::ModelOperation,
    ) -> crate::layout_engine::ModelOperationResult {
        let mut result = ModelOperationResult::NoCare;
        if let ModelOperation::ChangeDirection(direction) = op {
            result = ModelOperationResult::RequireReLayout;
            match direction {
                Some(direction) => self.direction = *direction,
                None => self.direction = self.direction.toggle(),
            }
        }

        for model in self.models.iter_mut() {
            if model.model_operation(op) != ModelOperationResult::NoCare {
                result = ModelOperationResult::RequireReLayout;
            }
        }
        result
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
            model.set_border(border);
        }
    }

    fn border(&self) -> crate::layout_engine::ModelBorder {
        crate::layout_engine::ModelBorder::None
    }

    fn set_easing_preset(&mut self, preset: crate::ui_context::CharEasingsPreset) {
        for model in self.models.iter_mut() {
            model.set_easing_preset(preset);
        }
    }
}
