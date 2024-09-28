mod categorized_memos_world;
mod help_world;

pub(crate) use categorized_memos_world::CategorizedMemosWorld;
pub(crate) use help_world::HelpWorld;

use std::collections::HashSet;

use font_rasterizer::{
    context::StateContext,
    layout_engine::{Model, World},
    support::InputResult,
};
use stroke_parser::Action;

pub(crate) trait ModalWorld {
    fn get_mut(&mut self) -> &mut dyn World;
    fn get(&self) -> &dyn World;
    fn apply_action(
        &mut self,
        context: &StateContext,
        action: Action,
    ) -> (InputResult, HashSet<char>);
    fn world_chars(&self) -> HashSet<char>;
    fn graceful_exit(&mut self);
    fn add_modal(
        &mut self,
        context: &StateContext,
        chars: &mut HashSet<char>,
        model: Box<dyn Model>,
    );
}
