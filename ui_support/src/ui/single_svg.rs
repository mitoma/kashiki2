use font_rasterizer::{
    color_theme::ThemedColor,
    vector_instances::{InstanceAttributes, InstanceKey, VectorInstances},
};
use glam::{Quat, Vec3};

use crate::{easing_value::EasingPointN, layout_engine::Model, ui_context::UiContext};

pub struct SingleSvg {
    svg_instance: VectorInstances<String>,

    position: EasingPointN<3>,
    rotation: EasingPointN<4>,
    instance_scale: [f32; 2],
    themed_color: ThemedColor,

    bound: EasingPointN<2>,
}

impl SingleSvg {
    pub fn new(svg_string: String, ui_context: &UiContext, themed_color: ThemedColor) -> Self {
        let key = "key";
        let mut svg_instance = VectorInstances::new(key.into(), ui_context.device());
        svg_instance.insert(
            InstanceKey::Monotonic(1),
            InstanceAttributes {
                ..Default::default()
            },
        );

        ui_context.register_svg(key.into(), svg_string);
        Self {
            svg_instance,
            themed_color,
            position: EasingPointN::new([0.0, 0.0, 0.0]),
            rotation: EasingPointN::new([0.0, 0.0, 0.0, 1.0]),
            instance_scale: [1.0, 1.0],
            bound: EasingPointN::new([0.0, 0.0]),
        }
    }
}

impl Model for SingleSvg {
    fn set_position(&mut self, position: Vec3) {
        let p: [f32; 3] = position.into();
        if self.position.last() == p {
            return;
        }
        self.position.update(position.into());
    }

    fn position(&self) -> Vec3 {
        self.position.current().into()
    }

    fn last_position(&self) -> Vec3 {
        self.position.last().into()
    }

    fn focus_position(&self) -> glam::Vec3 {
        self.position.last().into()
    }

    fn set_rotation(&mut self, rotation: Quat) {
        let [x, y, z, w] = rotation.to_array();
        if self.rotation.last() == [x, y, z, w] {
            return;
        }
        self.rotation.update([x, y, z, w]);
    }

    fn rotation(&self) -> Quat {
        Quat::from_array(self.rotation.last())
    }

    fn bound(&self) -> (f32, f32) {
        // 外向けにはアニメーション完了後の最終的なサイズを返す
        // この値はレイアウトの計算に使われるためである
        self.bound.last().into()
    }

    fn glyph_instances(&self) -> Vec<&font_rasterizer::glyph_instances::GlyphInstances> {
        vec![]
    }

    fn vector_instances(&self) -> Vec<&font_rasterizer::vector_instances::VectorInstances<String>> {
        vec![&self.svg_instance]
    }

    fn update(&mut self, context: &crate::ui_context::UiContext) {
        self.bound.update(self.instance_scale);
        let color = self.themed_color.get_color(context.color_theme());
        if let Some(attributes) = self.svg_instance.get_mut(&InstanceKey::Monotonic(1)) {
            attributes.position = self.position.current().into();
            attributes.rotation = Quat::from_array(self.rotation.current());
            attributes.color = color;
            attributes.instance_scale = self.instance_scale;
        }
        self.svg_instance
            .update_buffer(context.device(), context.queue());
    }

    fn editor_operation(&mut self, _op: &text_buffer::action::EditorOperation) {}

    fn model_operation(
        &mut self,
        _op: &crate::layout_engine::ModelOperation,
    ) -> crate::layout_engine::ModelOperationResult {
        crate::layout_engine::ModelOperationResult::NoCare
    }

    fn to_string(&self) -> String {
        String::new()
    }

    fn in_animation(&self) -> bool {
        self.position.in_animation() || self.rotation.in_animation() || self.bound.in_animation()
    }

    fn set_border(&mut self, _border: crate::layout_engine::ModelBorder) {}

    fn border(&self) -> crate::layout_engine::ModelBorder {
        crate::layout_engine::ModelBorder::None
    }

    fn set_easing_preset(&mut self, _preset: crate::ui_context::CharEasingsPreset) {}
}
