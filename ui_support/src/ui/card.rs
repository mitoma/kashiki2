use glam::Vec3;
use text_buffer::action::EditorOperation;

use font_rasterizer::{
    glyph_instances::GlyphInstances, glyph_vertex_buffer::Direction,
    vector_instances::VectorInstances,
};

use crate::editor_settings::{EditorSettings, EditorTextContextProfile};
use crate::ui_context::UiContext;

use crate::layout_engine::{Model, ModelBorder, ModelOperation, ModelSurfaceStyle};

use super::textedit::TextEdit;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum CardAnchor {
    // 位置を自動調整しない
    #[default]
    Free,
    // 画面左上に固定する
    TopLeft,
    // 画面右上に固定する
    TopRight,
    // 画面左下に固定する
    BottomLeft,
    // 画面右下に固定する
    BottomRight,
}

pub struct Card {
    text_edit: TextEdit,
    world_scale: [f32; 2],
    anchor: CardAnchor,
    direction: Direction,
    // 最も長い行の文字幅。行方向の縮小率の計算に使う
    max_line_width: f32,
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}

const CARD_DEFAULT_SCALE: [f32; 2] = [0.05, 0.05];
// 画面端からの余白(カメラを無視した座標系での値)
const CARD_SCREEN_MARGIN: f32 = 0.05;
// TextEdit の罫線と面は bound の約 1 セル外側まで描画されるため、その分だけ内側に寄せる
const CARD_BORDER_OUTSET: f32 = 1.0;

impl Card {
    pub fn new() -> Self {
        Self::with_settings(EditorSettings::default())
    }

    pub fn with_settings(editor_settings: EditorSettings) -> Self {
        let direction = editor_settings.global_direction();
        let mut text_edit =
            TextEdit::new(editor_settings.text_context(EditorTextContextProfile::Card));
        text_edit.set_world_scale(CARD_DEFAULT_SCALE);
        text_edit.set_position((0.0, -1.5, 0.0).into());
        text_edit.set_surface_style(ModelSurfaceStyle::OPAQUE_THEME_BACKGROUND);

        Self {
            text_edit,
            world_scale: CARD_DEFAULT_SCALE,
            anchor: CardAnchor::Free,
            direction,
            max_line_width: 0.0,
        }
    }

    pub fn set_anchor(&mut self, anchor: CardAnchor) {
        self.anchor = anchor;
    }

    pub fn set_border(&mut self, border: ModelBorder) {
        self.text_edit.set_border(border);
    }

    pub fn border(&self) -> ModelBorder {
        self.text_edit.border()
    }

    pub fn set_text(&mut self, context: &UiContext, text: String) {
        self.max_line_width = text
            .lines()
            .map(|line| {
                line.chars()
                    .map(|c| context.char_width_calcurator().get_width(c).to_f32())
                    .sum::<f32>()
            })
            .fold(0.0_f32, f32::max);
        self.update_world_scale(context.window_size().aspect());

        for op in [
            EditorOperation::Mark,
            EditorOperation::BufferHead,
            EditorOperation::Cut(|_| {}),
            EditorOperation::InsertString(text),
        ] {
            self.text_edit.editor_operation(&op);
        }
    }

    pub fn update(&mut self, context: &UiContext) {
        let direction = context.global_direction();
        if self.direction != direction {
            self.direction = direction;
            self.text_edit
                .model_operation(&ModelOperation::ChangeDirection(Some(direction)));
        }
        let aspect = context.window_size().aspect();
        self.update_world_scale(aspect);
        self.apply_anchor(aspect);
        self.text_edit.update(context)
    }

    // 長い行が画面の半分を超えないよう、行方向の縮小率だけを下げる。
    // カメラを無視した座標系は x: [-aspect, aspect], y: [-1, 1] なので、半分は x で aspect、y で 1.0 になる
    fn update_world_scale(&mut self, aspect: f32) {
        let [default_x, default_y] = CARD_DEFAULT_SCALE;
        let fit = |default: f32, half_screen: f32| {
            if self.max_line_width > 0.0 {
                default.min(half_screen / self.max_line_width)
            } else {
                default
            }
        };
        self.world_scale = match self.direction {
            Direction::Horizontal => [fit(default_x, aspect), default_y],
            Direction::Vertical => [default_x, fit(default_y, 1.0)],
        };
        self.text_edit.set_world_scale(self.world_scale);
    }

    pub fn get_instances(&self) -> Vec<&GlyphInstances> {
        self.text_edit.glyph_instances()
    }

    pub fn get_vector_instances(&self) -> Vec<&VectorInstances<String>> {
        self.text_edit.vector_instances()
    }

    pub fn get_surface_instances(&self) -> Vec<&VectorInstances<String>> {
        self.text_edit.surface_instances()
    }

    // カメラを無視した座標系は x: [-aspect, aspect], y: [-1, 1] で、
    // world_scale は位置にも掛かるため、画面上の位置を world_scale で割って求める。
    fn apply_anchor(&mut self, aspect: f32) {
        let (horizontal, vertical) = match self.anchor {
            CardAnchor::Free => return,
            CardAnchor::TopLeft => (-1.0, 1.0),
            CardAnchor::TopRight => (1.0, 1.0),
            CardAnchor::BottomLeft => (-1.0, -1.0),
            CardAnchor::BottomRight => (1.0, -1.0),
        };
        let (width, height) = self.text_edit.bound();
        let [scale_x, scale_y] = self.world_scale;
        let outset = self.border_outset();
        let x = horizontal * ((aspect - CARD_SCREEN_MARGIN) / scale_x - width / 2.0 - outset);
        let y = vertical * ((1.0 - CARD_SCREEN_MARGIN) / scale_y - height / 2.0 - outset);
        self.text_edit.set_position(Vec3::new(x, y, 0.0));
    }

    fn border_outset(&self) -> f32 {
        if self.border() == ModelBorder::None
            && self.text_edit.surface_style() == ModelSurfaceStyle::None
        {
            0.0
        } else {
            CARD_BORDER_OUTSET
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corner_anchors_keep_the_card_edges_inside_the_screen() {
        let cases = [
            (CardAnchor::TopLeft, -1.0, 1.0),
            (CardAnchor::TopRight, 1.0, 1.0),
            (CardAnchor::BottomLeft, -1.0, -1.0),
            (CardAnchor::BottomRight, 1.0, -1.0),
        ];
        let configurations = [
            (16.0 / 9.0, [0.05, 0.05], [4.0, 8.0]),
            (0.75, [0.08, 0.04], [9.0, 3.0]),
        ];
        // 面も罫線もなければ bound の外側には何も描画されないため、寄せる必要はない
        let surfaces = [
            (
                ModelSurfaceStyle::OPAQUE_THEME_BACKGROUND,
                CARD_BORDER_OUTSET,
            ),
            (ModelSurfaceStyle::None, 0.0),
        ];

        for ((aspect, scale, bound), (surface, outset)) in configurations
            .into_iter()
            .flat_map(|configuration| surfaces.map(|surface| (configuration, surface)))
        {
            let mut settings = EditorSettings::default();
            settings.profiles.card.min_bound = Some(bound);
            let mut card = Card::with_settings(settings);
            card.text_edit.set_surface_style(surface);
            card.world_scale = scale;
            card.text_edit.set_world_scale(scale);
            let (width, height) = card.text_edit.bound();

            for (anchor, horizontal, vertical) in cases {
                card.set_anchor(anchor);
                card.apply_anchor(aspect);
                let position = card.text_edit.last_position();
                let edge_x = (position.x + horizontal * (width / 2.0 + outset)) * scale[0];
                let edge_y = (position.y + vertical * (height / 2.0 + outset)) * scale[1];
                let expected_x = horizontal * (aspect - CARD_SCREEN_MARGIN);
                let expected_y = vertical * (1.0 - CARD_SCREEN_MARGIN);

                assert!(
                    (edge_x - expected_x).abs() < 1e-5,
                    "{anchor:?}: edge_x={edge_x}, expected={expected_x}"
                );
                assert!(
                    (edge_y - expected_y).abs() < 1e-5,
                    "{anchor:?}: edge_y={edge_y}, expected={expected_y}"
                );
                assert_eq!(position.z, 0.0);
            }
        }
    }

    #[test]
    fn corner_anchors_keep_the_border_inside_the_screen() {
        let aspect = 16.0 / 9.0;
        let scale = [0.05, 0.05];
        let mut settings = EditorSettings::default();
        settings.profiles.card.min_bound = Some([4.0, 8.0]);
        let mut card = Card::with_settings(settings);
        card.set_border(ModelBorder::Rounded);
        assert_eq!(card.border(), ModelBorder::Rounded);
        let (width, height) = card.text_edit.bound();

        card.set_anchor(CardAnchor::BottomRight);
        card.apply_anchor(aspect);
        let position = card.text_edit.last_position();
        let edge_x = (position.x + width / 2.0 + CARD_BORDER_OUTSET) * scale[0];
        let edge_y = (position.y - height / 2.0 - CARD_BORDER_OUTSET) * scale[1];

        assert!((edge_x - (aspect - CARD_SCREEN_MARGIN)).abs() < 1e-5);
        assert!((edge_y - -(1.0 - CARD_SCREEN_MARGIN)).abs() < 1e-5);
    }

    #[test]
    fn free_anchor_does_not_change_the_card_position() {
        let mut card = Card::new();
        let position = Vec3::new(1.25, -2.0, 0.5);
        card.text_edit.set_position(position);

        card.set_anchor(CardAnchor::Free);
        card.apply_anchor(16.0 / 9.0);

        assert_eq!(card.text_edit.last_position(), position);
    }
}
