use text_buffer::action::EditorOperation;

use font_rasterizer::glyph_instances::GlyphInstances;

use crate::editor_settings::{EditorSettings, EditorTextContextProfile};
use crate::ui_context::UiContext;

use crate::layout_engine::Model;

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
        Self::with_settings(EditorSettings::default())
    }

    pub fn with_settings(editor_settings: EditorSettings) -> Self {
        let mut text_edit =
            TextEdit::new(editor_settings.text_context(EditorTextContextProfile::Card));
        text_edit.set_world_scale(CARD_DEFAULT_SCALE);
        text_edit.set_position((0.0, -1.5, 0.0).into());

        Self { text_edit }
    }

    pub fn set_text(&mut self, context: &UiContext, text: String) {
        let char_width = text
            .chars()
            .map(|c| context.char_width_calcurator().get_width(c).to_f32())
            .sum::<f32>();
        self.text_edit.set_world_scale([
            f32::min(
                CARD_DEFAULT_SCALE[0],
                1.0 / char_width * context.window_size().aspect(),
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

    pub fn update(&mut self, context: &UiContext) {
        self.text_edit.update(context)
    }

    pub fn get_instances(&self) -> Vec<&GlyphInstances> {
        self.text_edit.glyph_instances()
    }
}
