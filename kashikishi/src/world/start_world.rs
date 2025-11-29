use std::collections::HashSet;

use font_rasterizer::glyph_vertex_buffer::Direction;
use ui_support::{
    InputResult,
    camera::CameraAdjustment,
    layout_engine::{DefaultWorld, Model, World},
    ui::{SelectBox, SelectOption},
    ui_context::UiContext,
};

use stroke_parser::Action;

use super::ModalWorld;

pub(crate) struct StartWorld {
    world: DefaultWorld,
}

impl StartWorld {
    pub(crate) fn new(context: &UiContext) -> Self {
        let mut result = Self {
            world: DefaultWorld::new(context.window_size()),
        };

        let options = vec![
            SelectOption::new(
                "メモ帳を開く".to_string(),
                Action::new_command("mode", "category"),
            ),
            SelectOption::new(
                "ヘルプ(使い方の概説)を開く".to_string(),
                Action::new_command("mode", "help"),
            ),
            SelectOption::new(
                "炊紙を終了する".to_string(),
                Action::new_command("system", "exit"),
            ),
        ];
        let start_select = SelectBox::new_without_action_name(
            context,
            "炊紙 kashikishi".to_string(),
            options,
            None,
        )
        .without_cancellable();

        result.world.add(Box::new(start_select));
        result.world.re_layout();
        result
    }
}

impl ModalWorld for StartWorld {
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
        self.world.chars()
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
