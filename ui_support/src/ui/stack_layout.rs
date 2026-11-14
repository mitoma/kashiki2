use font_rasterizer::{
    glyph_vertex_buffer::Direction,
    vector_instances::{InstanceAttributes, VectorInstances},
};
use glam::Quat;

use crate::{
    easing_value::EasingPointN,
    layout_engine::{
        DebugModelDetails, DebugModelNode, DebugStackLayoutSnapshot, Model, ModelAttributes,
        ModelBorder, ModelOperation, ModelOperationResult, ModelSurfaceStyle,
    },
    ui::view_element_state::{
        BORDER_OUTSET, BorderStates, SURFACE_OUTSET, SurfaceGeometry, surface_corner_radius,
        update_model_surface_instances,
    },
    ui_context::TextContext,
};

/// Model を複数まとめてレイアウトするためのコンテナ
/// Direction に応じて子モデルを縦または横に並べる
pub struct StackLayout {
    direction: Direction,

    models: Vec<Box<dyn Model>>,
    focus_model_index: Option<usize>,
    focus_to_model: bool,

    position: EasingPointN<3>,
    rotation: EasingPointN<4>,
    // 枠線と面の描画用。子モデルの配置と bound() は目標値の content_bound を使う
    content_bound: Option<EasingPointN<2>>,

    margin: Margin,

    // 子モデルごとではなく、レイアウト全体を囲む枠線
    border: ModelBorder,
    border_states: Option<BorderStates>,
    border_context: TextContext,
    surface_style: ModelSurfaceStyle,
    surface_instances: Option<VectorInstances<String>>,
    surface_geometry: SurfaceGeometry,
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
            focus_to_model: false,
            position: EasingPointN::new([0.0, 0.0, 0.0]),
            rotation: EasingPointN::new([0.0, 0.0, 0.0, 1.0]),
            content_bound: None,
            margin: Margin {
                horizontal: 0.1,
                vertical: 0.1,
            },
            border: ModelBorder::None,
            border_states: None,
            border_context: TextContext::default(),
            surface_style: ModelSurfaceStyle::None,
            surface_instances: None,
            surface_geometry: SurfaceGeometry::default(),
        }
    }

    fn update_surface(&mut self, context: &crate::ui_context::UiContext) {
        if self.models.is_empty() || self.surface_style == ModelSurfaceStyle::None {
            self.surface_instances = None;
            return;
        }
        let outset = SURFACE_OUTSET * 2.0;
        let ([last_width, last_height], [current_width, current_height]) =
            self.animated_content_bound();
        let (geometry_key, instance_scale) = self.surface_geometry.resolve(
            [last_width + outset, last_height + outset],
            [current_width + outset, current_height + outset],
            surface_corner_radius(self.border),
            context,
        );
        let attributes = InstanceAttributes {
            position: self.position.current().into(),
            rotation: Quat::from_array(self.rotation.current()),
            instance_scale,
            ..Default::default()
        };
        update_model_surface_instances(
            &mut self.surface_instances,
            self.surface_style,
            &geometry_key,
            attributes,
            context,
        );
    }

    fn update_border(&mut self, context: &crate::ui_context::UiContext) {
        if self.border == ModelBorder::None {
            return;
        }
        self.border_context.color_theme = *context.color_theme();
        // BorderStates はモデルの左上を原点とした座標で、内容の外側に枠線を配置する。
        // TextEdit と同様に、罫線の各片は目標の bound へイージングし、中心は現在の bound に追従させる
        let ([bound_x, bound_y], [current_x, current_y]) = self.animated_content_bound();
        let model_attributes = ModelAttributes {
            center: (current_x / 2.0, -current_y / 2.0).into(),
            position: self.position.current().into(),
            rotation: Quat::from_array(self.rotation.current()),
            world_scale: [1.0, 1.0],
        };
        let border_context = &self.border_context;
        let states = self.border_states.get_or_insert_with(|| {
            let mut states = BorderStates::default();
            states.init(border_context, context.device());
            states
        });
        states.update_state([0.0, 0.0, 0.0], [bound_x, bound_y], border_context);
        states.update_instances(true, &model_attributes);
        states.instances.update(context.device(), context.queue());
    }

    // 初回は目標値から始め、モーダルを開いた瞬間に面が 0 から広がらないようにする
    fn update_content_bound(&mut self) {
        let target: [f32; 2] = self.content_bound().into();
        match self.content_bound.as_mut() {
            Some(bound) => bound.update(target),
            None => self.content_bound = Some(EasingPointN::new(target)),
        }
    }

    // (目標値, 現在値)
    fn animated_content_bound(&self) -> ([f32; 2], [f32; 2]) {
        match self.content_bound.as_ref() {
            Some(bound) => (bound.last(), bound.current()),
            None => {
                let target = self.content_bound().into();
                (target, target)
            }
        }
    }

    // 枠線を含まない、子モデルを並べた部分の大きさ
    fn content_bound(&self) -> (f32, f32) {
        match self.direction {
            Direction::Horizontal => (
                // 横書き: テキスト行が下に伸びる → 複数行を縦に積む
                self.models.iter().map(|m| m.bound().0).fold(0.0, f32::max),
                self.models.iter().map(|m| m.bound().1).sum::<f32>()
                    + self.margin.vertical * (self.models.len() as f32 - 1.0),
            ),
            Direction::Vertical => (
                // 縦書き: テキスト列が右に伸びる → 複数列を横に並べる
                self.models.iter().map(|m| m.bound().0).sum::<f32>()
                    + self.margin.horizontal * (self.models.len() as f32 - 1.0),
                self.models.iter().map(|m| m.bound().1).fold(0.0, f32::max),
            ),
        }
    }

    pub fn add_model(&mut self, mut model: Box<dyn Model>) {
        model.model_operation(&ModelOperation::ChangeDirection(Some(self.direction)));
        self.models.push(model);
    }

    pub fn set_focus_model_index(&mut self, index: usize, focus_to_model: bool) {
        self.focus_model_index = Some(index);
        self.focus_to_model = focus_to_model;
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

    pub fn margin(&self) -> (f32, f32) {
        (self.margin.horizontal, self.margin.vertical)
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
        if self.focus_to_model
            && let Some(index) = self.focus_model_index
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

    // 枠線か面があるときは、カメラがそれらまで画面に収められるよう外側の分も含める
    fn bound(&self) -> (f32, f32) {
        let (x, y) = self.content_bound();
        let outset = if self.border == ModelBorder::None
            && (self.surface_style == ModelSurfaceStyle::None || self.models.is_empty())
        {
            0.0
        } else {
            BORDER_OUTSET * 2.0
        };
        (x + outset, y + outset)
    }

    fn surface_instances(
        &self,
    ) -> Vec<&font_rasterizer::vector_instances::VectorInstances<String>> {
        self.surface_instances
            .iter()
            .chain(
                self.models
                    .iter()
                    .flat_map(|model| model.surface_instances()),
            )
            .collect()
    }

    fn set_surface_style(&mut self, surface_style: ModelSurfaceStyle) {
        self.surface_style = surface_style;
    }

    fn surface_style(&self) -> ModelSurfaceStyle {
        self.surface_style
    }

    fn glyph_instances(&self) -> Vec<&font_rasterizer::glyph_instances::GlyphInstances> {
        self.models
            .iter()
            .flat_map(|model| model.glyph_instances())
            .collect()
    }

    fn vector_instances(&self) -> Vec<&font_rasterizer::vector_instances::VectorInstances<String>> {
        let mut result: Vec<_> = self
            .models
            .iter()
            .flat_map(|model| model.vector_instances())
            .collect();
        if let Some(states) = self.border_states.as_ref() {
            result.extend(states.instances.to_instances());
        }
        result
    }

    fn update(&mut self, context: &crate::ui_context::UiContext) {
        let position: glam::Vec3 = self.last_position();
        let layout_bound: (f32, f32) = self.content_bound();
        //log::info!("--------");
        //log::info!(
        //    "StackLayout update: position={:?}, bound={:?}",
        //    position,
        //    layout_bound
        //);
        match self.direction {
            Direction::Horizontal => {
                let mut current_y = position.y + layout_bound.1 / 2.0;
                for model in self.models.iter_mut() {
                    let bound = model.bound();
                    current_y -= bound.1 / 2.0;
                    //log::info!(
                    //    "model bound: {:?}, model_position: x={}, y={}",
                    //    bound,
                    //    position.x,
                    //    current_y
                    //);
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
                    //log::info!(
                    //    "model bound: {:?}, model_position: x={}, y={}",
                    //    bound,
                    //    current_x,
                    //    position.y
                    //);
                    model.set_position(glam::vec3(current_x, position.y, position.z));
                    current_x -= (bound.0 + self.margin.horizontal) / 2.0;
                }
            }
        }

        for model in self.models.iter_mut() {
            model.update(context);
        }
        self.update_content_bound();
        self.update_border(context);
        self.update_surface(context);
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
        // 枠線は子モデルに配らず、レイアウト全体に 1 つだけ付ける
        if let ModelOperation::SetModelBorder(border) = op {
            self.set_border(*border);
            return ModelOperationResult::RequireReLayoutAndFit;
        }

        let mut result = ModelOperationResult::NoCare;
        if let ModelOperation::ChangeDirection(direction) = op {
            log::info!("StackLayout ChangeDirection: {:?}", direction);

            result = ModelOperationResult::RequireReLayout;
            match direction {
                Some(direction) => self.direction = *direction,
                None => self.direction = self.direction.toggle(),
            }
        }

        for (idx, model) in self.models.iter_mut().enumerate() {
            let should_apply =
                !matches!(op, ModelOperation::SetPreedit(_)) || Some(idx) == self.focus_model_index;

            if should_apply && model.model_operation(op) != ModelOperationResult::NoCare {
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
            || self
                .content_bound
                .as_ref()
                .is_some_and(|bound| bound.in_animation())
    }

    fn set_border(&mut self, border: ModelBorder) {
        self.border = border;
        if border == ModelBorder::None {
            self.border_states = None;
        }
    }

    fn border(&self) -> ModelBorder {
        self.border
    }

    fn set_easing_preset(&mut self, preset: crate::ui_context::CharEasingsPreset) {
        for model in self.models.iter_mut() {
            model.set_easing_preset(preset);
        }
    }

    fn debug_node(&self, camera: &crate::camera::Camera) -> DebugModelNode {
        let children = self
            .models
            .iter()
            .map(|model| model.debug_node(camera))
            .collect();
        let position = self.position().to_array();
        let last_position = self.last_position().to_array();
        let focus_position = self.focus_position().to_array();
        let rotation = self.rotation().to_array();
        let bound: [f32; 2] = self.bound().into();
        DebugModelNode::new(
            "StackLayout",
            self.border(),
            position,
            last_position,
            focus_position,
            rotation,
            bound,
            bound,
            self.in_animation(),
            children,
            DebugModelDetails::StackLayout(DebugStackLayoutSnapshot {
                direction: match self.direction {
                    Direction::Horizontal => "horizontal",
                    Direction::Vertical => "vertical",
                },
                margin: [self.margin.horizontal, self.margin.vertical],
                focus_model_index: self.focus_model_index,
                focus_to_model: self.focus_to_model,
            }),
            camera,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::TextEdit;

    #[test]
    fn surface_belongs_to_the_whole_layout_without_changing_child_styles() {
        for direction in [Direction::Horizontal, Direction::Vertical] {
            let mut layout = StackLayout::new(direction);
            let child_style = ModelSurfaceStyle::Color([1.0, 0.0, 0.0, 0.5]);
            let mut child = TextEdit::default();
            child.set_surface_style(child_style);
            layout.add_model(Box::new(child));

            assert_eq!(layout.surface_style(), ModelSurfaceStyle::None);
            layout.set_surface_style(ModelSurfaceStyle::ThemeBackground { alpha: 0.9 });
            layout.add_model(Box::new(TextEdit::default()));

            assert_eq!(
                layout.surface_style(),
                ModelSurfaceStyle::ThemeBackground { alpha: 0.9 }
            );
            assert_eq!(layout.models()[0].surface_style(), child_style);
            assert_eq!(layout.models()[1].surface_style(), ModelSurfaceStyle::None);

            layout.set_surface_style(ModelSurfaceStyle::None);
            assert_eq!(layout.models()[0].surface_style(), child_style);
        }
    }

    #[test]
    fn surface_without_border_keeps_the_rounded_outline_inside_the_bound() {
        let mut layout = StackLayout::new(Direction::Horizontal);
        layout.add_model(Box::new(TextEdit::default()));
        let (width, height) = layout.bound();

        layout.set_surface_style(ModelSurfaceStyle::OPAQUE_THEME_BACKGROUND);

        assert_eq!(layout.border(), ModelBorder::None);
        assert_eq!(
            layout.bound(),
            (width + BORDER_OUTSET * 2.0, height + BORDER_OUTSET * 2.0)
        );
        assert!(surface_corner_radius(layout.border()) > 0.0);
    }

    #[test]
    fn border_wraps_the_whole_layout_instead_of_each_child() {
        let mut layout = StackLayout::new(Direction::Horizontal);
        layout.add_model(Box::new(TextEdit::default()));
        layout.add_model(Box::new(TextEdit::default()));
        let (width, height) = layout.bound();

        let result = layout.model_operation(&ModelOperation::SetModelBorder(ModelBorder::Rounded));

        assert!(result == ModelOperationResult::RequireReLayoutAndFit);
        assert_eq!(layout.border(), ModelBorder::Rounded);
        assert!(
            layout
                .models()
                .iter()
                .all(|model| model.border() == ModelBorder::None)
        );
        assert_eq!(
            layout.bound(),
            (width + BORDER_OUTSET * 2.0, height + BORDER_OUTSET * 2.0)
        );

        layout.model_operation(&ModelOperation::SetModelBorder(ModelBorder::None));
        assert_eq!(layout.bound(), (width, height));
    }

    #[test]
    fn surface_and_border_bound_eases_while_layout_bound_uses_the_target() {
        let mut layout = StackLayout::new(Direction::Horizontal);
        layout.add_model(Box::new(TextEdit::default()));
        layout.update_content_bound();
        let initial: [f32; 2] = layout.content_bound().into();
        assert_eq!(layout.animated_content_bound(), (initial, initial));

        layout.add_model(Box::new(TextEdit::default()));
        layout.update_content_bound();
        let target: [f32; 2] = layout.content_bound().into();
        let (last, current) = layout.animated_content_bound();

        assert!(target[1] > initial[1]);
        assert_eq!(last, target);
        assert!(current[1] < target[1]);
        assert_eq!(layout.bound(), target.into());
        assert!(layout.in_animation());
    }
}
