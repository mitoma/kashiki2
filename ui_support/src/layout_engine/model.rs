use std::sync::Arc;

use cgmath::{Matrix4, Point2, Point3, Quaternion};
use text_buffer::{action::EditorOperation, editor::CharWidthResolver};

use font_rasterizer::{
    context::StateContext, glyph_instances::GlyphInstances, glyph_vertex_buffer::Direction,
    vector_instances::VectorInstances,
};

#[derive(Default, Clone, Copy, PartialEq)]
pub enum ModelBorder {
    #[default]
    None,
    Square,
    Rounded,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttributeType {
    Current,
    Last,
}

pub trait Model {
    // モデルの位置を設定する
    fn set_position(&mut self, position: Point3<f32>);
    // モデルの位置を返す
    fn position(&self, attribute_type: AttributeType) -> Point3<f32>;
    // モデル中、カメラがフォーカスすべき位置を返す
    // position はモデルの中心を指す
    fn focus_position(&self) -> Point3<f32>;
    // モデルの回転を設定する
    fn set_rotation(&mut self, rotation: Quaternion<f32>);
    // モデルの回転を返す
    fn rotation(&self, attribute_type: AttributeType) -> Quaternion<f32>;
    // モデルの縦横の長さを返す
    fn bound(&self, attribute_type: AttributeType) -> (f32, f32);
    fn glyph_instances(&self) -> Vec<&GlyphInstances>;
    fn vector_instances(&self) -> Vec<&VectorInstances<String>>;
    fn update(&mut self, context: &StateContext);
    fn editor_operation(&mut self, op: &EditorOperation);
    fn model_operation(&mut self, op: &ModelOperation) -> ModelOperationResult;
    fn to_string(&self) -> String;
    //fn model_mode(&self) -> ModelMode;
    fn in_animation(&self) -> bool;
    fn set_border(&mut self, border: ModelBorder);
    fn border(&self) -> ModelBorder;
    fn set_world_scale(&mut self, world_scale: Scale);
    fn world_scale(&self) -> Scale;

    #[inline]
    fn model_attributes(&self) -> ModelAttributes {
        let attribute_type = AttributeType::Current;
        let (bound_x, bound_y) = &self.bound(attribute_type);
        let center = (bound_x / 2.0, -bound_y / 2.0).into();
        let current_position: Point3<f32> = self.position(attribute_type);
        ModelAttributes {
            position: current_position,
            rotation: self.rotation(attribute_type),
            center,
            scale: self.world_scale(),
        }
    }
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
    SetModelBorder(ModelBorder),
    SetMaxCol(usize),
    IncreaseMaxCol,
    DecreaseMaxCol,
    ToggleHighlightMode,
}

pub enum ModelOperationResult {
    NoCare,
    RequireReLayout,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Scale {
    pub width: f32,
    pub height: f32,
}

impl Default for Scale {
    fn default() -> Self {
        Scale {
            width: 1.0,
            height: 1.0,
        }
    }
}

impl From<[f32; 2]> for Scale {
    #[inline]
    fn from(arr: [f32; 2]) -> Self {
        Scale {
            width: arr[0],
            height: arr[1],
        }
    }
}

impl From<Scale> for [f32; 2] {
    #[inline]
    fn from(scale: Scale) -> Self {
        [scale.width, scale.height]
    }
}

// モデルが持つ属性をまとめたもの
pub struct ModelAttributes {
    pub center: Point2<f32>,
    // モデルの world 空間上の位置
    pub position: Point3<f32>,
    // モデルの world 空間上の回転(向き)
    pub rotation: Quaternion<f32>,
    // モデルの world 空間上の拡大率(x, y)
    pub scale: Scale,
}
