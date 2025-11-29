use std::collections::HashSet;

use font_rasterizer::{context::WindowSize, glyph_vertex_buffer::Direction};
use ui_support::{
    InputResult,
    camera::CameraAdjustment,
    layout_engine::{DefaultWorld, Model, World},
    ui_context::UiContext,
};

use stroke_parser::Action;

use super::ModalWorld;

/// 特に何もしない World
/// Null Object Pattern の目的で用意されている。
pub(crate) struct NullWorld {
    world: DefaultWorld,
}

impl NullWorld {
    pub(crate) fn new(window_size: WindowSize) -> Self {
        Self {
            world: DefaultWorld::new(window_size),
        }
    }
}

impl ModalWorld for NullWorld {
    fn get_mut(&mut self) -> &mut dyn World {
        &mut self.world
    }

    fn get(&self) -> &dyn World {
        &self.world
    }

    fn apply_action(
        &mut self,
        _context: &UiContext,
        _action: Action,
    ) -> (InputResult, HashSet<char>) {
        (InputResult::Noop, HashSet::new())
    }

    fn world_chars(&self) -> HashSet<char> {
        HashSet::new()
    }

    fn graceful_exit(&mut self) {
        // noop
    }

    fn add_modal(&mut self, context: &UiContext, chars: &mut HashSet<char>, model: Box<dyn Model>) {
        chars.extend(model.to_string().chars());
        self.world.add_next(model);
        self.world.re_layout();
        let adjustment = if context.global_direction() == Direction::Horizontal {
            CameraAdjustment::FitWidth
        } else {
            CameraAdjustment::FitHeight
        };
        self.world.look_next(adjustment);
    }
}
