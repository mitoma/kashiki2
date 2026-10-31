mod categorized_memos_world;
mod help_world;
mod markdown_presentation_world;
mod null_world;
mod start_world;
mod tagbase_memos_world;

pub(crate) use categorized_memos_world::CategorizedMemosWorld;
pub(crate) use help_world::HelpWorld;
pub(crate) use markdown_presentation_world::MarkdownPresentationWorld;
pub(crate) use null_world::NullWorld;
pub(crate) use start_world::StartWorld;
pub(crate) use tagbase_memos_world::{TagbaseMemosWorld, use_fixed_tagbase_clock};

use std::collections::HashSet;

use font_rasterizer::glyph_instances::GlyphInstances;
use ui_support::{
    InputResult,
    layout_engine::{Model, World},
    ui_context::UiContext,
};

use stroke_parser::Action;

pub(crate) trait ModalWorld {
    fn command_namespace(&self) -> Option<&'static str> {
        None
    }
    fn get_mut(&mut self) -> &mut dyn World;
    fn get(&self) -> &dyn World;
    // ワールド表示直後にカメラを合わせるためのアクション
    fn initial_camera_action(&self) -> Action {
        Action::new_command("world", "fit-by-direction")
    }
    fn update(&mut self, context: &UiContext) {
        self.get_mut().update(context);
    }
    // World のレイアウトやカメラの影響を受けず、常に画面上に重ねて表示するインスタンス
    fn overlay_glyph_instances(&self) -> Vec<&GlyphInstances> {
        Vec::new()
    }
    fn apply_action(&mut self, context: &UiContext, action: Action)
    -> (InputResult, HashSet<char>);
    fn world_chars(&self) -> HashSet<char>;
    fn graceful_exit(&mut self) {
        // noop
    }
    fn add_modal(&mut self, context: &UiContext, chars: &mut HashSet<char>, model: Box<dyn Model>);
}
