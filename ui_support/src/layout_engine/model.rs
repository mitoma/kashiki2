use std::sync::Arc;

use glam::{Mat4, Quat, Vec2, Vec3};
use serde::Serialize;
use text_buffer::{action::EditorOperation, editor::CharWidthResolver};

use font_rasterizer::{
    glyph_instances::GlyphInstances, glyph_vertex_buffer::Direction,
    vector_instances::VectorInstances,
};

use crate::{
    camera::Camera,
    ui_context::{CharEasingsPreset, UiContext},
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize)]
pub enum ModelBorder {
    #[default]
    None,
    Square,
    Rounded,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebugModelNode {
    pub name: &'static str,
    pub border: ModelBorder,
    pub position: [f32; 3],
    pub last_position: [f32; 3],
    pub focus_position: [f32; 3],
    pub rotation: [f32; 4],
    pub bound: [f32; 2],
    pub current_bound: [f32; 2],
    pub in_animation: bool,
    pub projected_center_ndc: [f32; 2],
    pub projected_quad_ndc: [[f32; 2]; 4],
    pub children: Vec<DebugModelNode>,
    pub details: DebugModelDetails,
}

impl DebugModelNode {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: &'static str,
        border: ModelBorder,
        position: [f32; 3],
        last_position: [f32; 3],
        focus_position: [f32; 3],
        rotation: [f32; 4],
        bound: [f32; 2],
        current_bound: [f32; 2],
        in_animation: bool,
        children: Vec<DebugModelNode>,
        details: DebugModelDetails,
        camera: &Camera,
    ) -> Self {
        let (projected_center_ndc, projected_quad_ndc) =
            project_model_quad(camera, position, rotation, current_bound);
        Self {
            name,
            border,
            position,
            last_position,
            focus_position,
            rotation,
            bound,
            current_bound,
            in_animation,
            projected_center_ndc,
            projected_quad_ndc,
            children,
            details,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum DebugModelDetails {
    None,
    StackLayout(DebugStackLayoutSnapshot),
    TextEdit(DebugTextEditSnapshot),
    TextInput(DebugTextInputSnapshot),
    SelectBox(DebugSelectBoxSnapshot),
    SingleSvg(DebugSingleSvgSnapshot),
}

#[derive(Debug, Clone, Serialize)]
pub struct DebugStackLayoutSnapshot {
    pub direction: &'static str,
    pub margin: [f32; 2],
    pub focus_model_index: Option<usize>,
    pub focus_to_model: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebugTextEditSnapshot {
    pub direction: &'static str,
    pub row_interval: f32,
    pub col_interval: f32,
    pub row_scale: f32,
    pub col_scale: f32,
    pub max_col: usize,
    pub min_bound: [f32; 2],
    pub instance_scale: [f32; 2],
    pub text: String,
    pub line_count: usize,
    pub char_count: usize,
    pub highlight_mode: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebugTextInputSnapshot {
    pub default_input: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebugSelectBoxSnapshot {
    pub current_selection: usize,
    pub option_count: usize,
    pub narrowed_option_count: usize,
    pub show_action_name: bool,
    pub cancellable: bool,
    pub max_line: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebugSingleSvgSnapshot {
    pub instance_scale: [f32; 2],
}

fn short_type_name<T: ?Sized>() -> &'static str {
    std::any::type_name::<T>()
        .rsplit("::")
        .next()
        .unwrap_or(std::any::type_name::<T>())
}

fn project_model_quad(
    camera: &Camera,
    position: [f32; 3],
    rotation: [f32; 4],
    current_bound: [f32; 2],
) -> ([f32; 2], [[f32; 2]; 4]) {
    let position = Vec3::from_array(position);
    let rotation = Quat::from_array(rotation);
    let transform = Mat4::from_translation(position) * Mat4::from_quat(rotation);
    let view_projection_matrix = camera.build_view_projection_matrix();

    let project = |point: Vec3| {
        let clip = view_projection_matrix * transform * point.extend(1.0);
        [clip.x / clip.w, clip.y / clip.w]
    };

    let [width, height] = current_bound;
    let half_width = width / 2.0;
    let half_height = height / 2.0;
    let center = project(Vec3::ZERO);
    let quad = [
        project(Vec3::new(-half_width, half_height, 0.0)),
        project(Vec3::new(half_width, half_height, 0.0)),
        project(Vec3::new(half_width, -half_height, 0.0)),
        project(Vec3::new(-half_width, -half_height, 0.0)),
    ];
    (center, quad)
}

pub trait Model {
    // モデルの位置を設定する
    fn set_position(&mut self, position: Vec3);
    // モデルの位置を返す
    fn position(&self) -> Vec3;
    // モデルの最終的な位置を返す(アニメーション中はアニメーション後の位置)
    fn last_position(&self) -> Vec3;
    // position はモデルの中心位置を指すのに対し、focus_position はモデル中、カメラがフォーカスすべき位置を返す
    fn focus_position(&self) -> Vec3;
    // モデルの回転を設定する
    fn set_rotation(&mut self, rotation: Quat);
    // モデルの回転を返す
    fn rotation(&self) -> Quat;
    // モデルの縦横の長さを返す
    fn bound(&self) -> (f32, f32);
    fn glyph_instances(&self) -> Vec<&GlyphInstances>;
    fn vector_instances(&self) -> Vec<&VectorInstances<String>>;
    fn update(&mut self, context: &UiContext);
    fn editor_operation(&mut self, op: &EditorOperation);
    fn model_operation(&mut self, op: &ModelOperation) -> ModelOperationResult;
    fn to_string(&self) -> String;
    //fn model_mode(&self) -> ModelMode;
    fn in_animation(&self) -> bool;
    fn set_border(&mut self, border: ModelBorder);
    fn border(&self) -> ModelBorder;
    fn set_easing_preset(&mut self, preset: CharEasingsPreset);
    fn debug_node(&self, camera: &Camera) -> DebugModelNode {
        let position = self.position().to_array();
        let last_position = self.last_position().to_array();
        let focus_position = self.focus_position().to_array();
        let rotation = self.rotation().to_array();
        let bound: [f32; 2] = self.bound().into();
        DebugModelNode::new(
            short_type_name::<Self>(),
            self.border(),
            position,
            last_position,
            focus_position,
            rotation,
            bound,
            bound,
            self.in_animation(),
            vec![],
            DebugModelDetails::None,
            camera,
        )
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
    MoveToClick(f32, f32, Mat4),
    MarkAndClick(f32, f32, Mat4),
    SetModelBorder(ModelBorder),
    SetMaxCol(usize),
    IncreaseMaxCol,
    DecreaseMaxCol,
    ToggleHighlightMode,
    // IME のプレエディット（未確定文字列）をモデルへ設定/解除する
    // None で解除、Some((value, selection)) で設定
    SetPreedit(Option<(String, Option<(usize, usize)>)>),
}

#[derive(PartialEq)]
pub enum ModelOperationResult {
    NoCare,
    RequireReLayout,
}

// モデルが持つ属性をまとめたもの
pub struct ModelAttributes {
    pub center: Vec2,
    // モデルの world 空間上の位置
    pub position: Vec3,
    // モデルの world 空間上の回転(向き)
    pub rotation: Quat,
    // モデルの world 空間上の拡大率(x, y)
    pub world_scale: [f32; 2],
}
