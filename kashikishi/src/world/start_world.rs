use std::collections::HashSet;

use font_rasterizer::{color_theme::ThemedColor, glyph_vertex_buffer::Direction};
use ui_support::{
    InputResult,
    camera::CameraAdjustment,
    layout_engine::{DefaultWorld, Model, World},
    ui::{SelectBox, SelectOption, SingleSvg, StackLayout},
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

        let mut layout = StackLayout::new(context.global_direction());

        let logo = SingleSvg::new(
            include_str!("../../../ui_support/asset/kashikishi-icon-toon-flat.svg").to_string(),
            context,
            ThemedColor::Blue,
        );
        layout.add_model(Box::new(logo));

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
        layout.add_model(Box::new(start_select));
        layout.set_focus_model_index(1);

        result.world.add(Box::new(layout));
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
