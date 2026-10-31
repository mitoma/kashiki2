use glam::Vec3;
use text_buffer::action::EditorOperation;

use font_rasterizer::glyph_instances::GlyphInstances;

use crate::editor_settings::{EditorSettings, EditorTextContextProfile};
use crate::ui_context::UiContext;

use crate::layout_engine::Model;

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
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}

const CARD_DEFAULT_SCALE: [f32; 2] = [0.05, 0.05];
// 画面端からの余白(カメラを無視した座標系での値)
const CARD_SCREEN_MARGIN: f32 = 0.05;

impl Card {
    pub fn new() -> Self {
        Self::with_settings(EditorSettings::default())
    }

    pub fn with_settings(editor_settings: EditorSettings) -> Self {
        let mut text_edit =
            TextEdit::new(editor_settings.text_context(EditorTextContextProfile::Card));
        text_edit.set_world_scale(CARD_DEFAULT_SCALE);
        text_edit.set_position((0.0, -1.5, 0.0).into());

        Self {
            text_edit,
            world_scale: CARD_DEFAULT_SCALE,
            anchor: CardAnchor::Free,
        }
    }

    pub fn set_anchor(&mut self, anchor: CardAnchor) {
        self.anchor = anchor;
    }

    pub fn set_text(&mut self, context: &UiContext, text: String) {
        let char_width = text
            .lines()
            .map(|line| {
                line.chars()
                    .map(|c| context.char_width_calcurator().get_width(c).to_f32())
                    .sum::<f32>()
            })
            .fold(0.0_f32, f32::max);
        self.world_scale = [
            f32::min(
                CARD_DEFAULT_SCALE[0],
                1.0 / char_width * context.window_size().aspect(),
            ),
            CARD_DEFAULT_SCALE[1],
        ];
        self.text_edit.set_world_scale(self.world_scale);

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
        self.apply_anchor(context.window_size().aspect());
        self.text_edit.update(context)
    }

    pub fn get_instances(&self) -> Vec<&GlyphInstances> {
        self.text_edit.glyph_instances()
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
        let x = horizontal * ((aspect - CARD_SCREEN_MARGIN) / scale_x - width / 2.0);
        let y = vertical * ((1.0 - CARD_SCREEN_MARGIN) / scale_y - height / 2.0);
        self.text_edit.set_position(Vec3::new(x, y, 0.0));
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

        for (aspect, scale, bound) in configurations {
            let mut settings = EditorSettings::default();
            settings.profiles.card.min_bound = Some(bound);
            let mut card = Card::with_settings(settings);
            card.world_scale = scale;
            card.text_edit.set_world_scale(scale);
            let (width, height) = card.text_edit.bound();

            for (anchor, horizontal, vertical) in cases {
                card.set_anchor(anchor);
                card.apply_anchor(aspect);
                let position = card.text_edit.last_position();
                let edge_x = (position.x + horizontal * width / 2.0) * scale[0];
                let edge_y = (position.y + vertical * height / 2.0) * scale[1];
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
    fn free_anchor_does_not_change_the_card_position() {
        let mut card = Card::new();
        let position = Vec3::new(1.25, -2.0, 0.5);
        card.text_edit.set_position(position);

        card.set_anchor(CardAnchor::Free);
        card.apply_anchor(16.0 / 9.0);

        assert_eq!(card.text_edit.last_position(), position);
    }
}
