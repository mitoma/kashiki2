use cgmath::Point2;
use text_buffer::action::EditorOperation;

use font_rasterizer::{context::StateContext, glyph_instances::GlyphInstances};

use crate::{
    layout_engine::Model,
    ui_context::{CharEasings, TextContext},
};

use super::textedit::TextEdit;

pub struct Card {
    text_edit: TextEdit,
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}

const CARD_DEFAULT_SCALE: [f32; 2] = [0.1, 0.1];

impl Card {
    pub fn new() -> Self {
        let config = TextContext {
            char_easings: CharEasings::ignore_camera(),
            max_col: usize::MAX,
            min_bound: Point2::new(1.0, 10.0),
            hyde_caret: true,
            ..Default::default()
        };
        let mut text_edit = TextEdit::default();
        text_edit.set_config(config);
        text_edit.set_world_scale(CARD_DEFAULT_SCALE);
        text_edit.set_position((0.0, -1.5, 0.0).into());

        Self { text_edit }
    }

    pub fn set_text(&mut self, context: &StateContext, text: String) {
        let char_width = text
            .chars()
            .map(|c| context.char_width_calcurator.get_width(c).to_f32())
            .sum::<f32>();
        self.text_edit.set_world_scale([
            f32::min(
                CARD_DEFAULT_SCALE[0],
                1.0 / char_width * context.window_size.aspect(),
            ),
            CARD_DEFAULT_SCALE[1],
        ]);

        for op in [
            EditorOperation::Mark,
            EditorOperation::BufferHead,
            EditorOperation::Cut(|_| {}),
            EditorOperation::InsertString(text),
        ] {
            self.text_edit.editor_operation(&op);
        }
    }

    pub fn update(&mut self, context: &StateContext) {
        self.text_edit.update(context)
    }

    pub fn get_instances(&self) -> Vec<&GlyphInstances> {
        self.text_edit.glyph_instances()
    }
}
