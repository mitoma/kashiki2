use std::collections::HashSet;

use font_rasterizer::{
    context::{StateContext, WindowSize},
    font_buffer::Direction,
};
use ui_support::{
    camera::CameraAdjustment,
    layout_engine::{HorizontalWorld, Model, World},
    ui::TextEdit,
    InputResult,
};

use stroke_parser::Action;
use text_buffer::action::EditorOperation;

use super::ModalWorld;

pub(crate) struct HelpWorld {
    world: HorizontalWorld,
}

impl HelpWorld {
    pub(crate) fn new(window_size: WindowSize) -> Self {
        let mut result = Self {
            world: HorizontalWorld::new(window_size),
        };

        let help_contents: Vec<String> =
            serde_json::from_str(include_str!("../../asset/help.json")).unwrap();

        for help_content in help_contents {
            let mut textedit = TextEdit::default();
            textedit.editor_operation(&EditorOperation::InsertString(help_content));
            textedit.editor_operation(&EditorOperation::BufferHead);
            let model = Box::new(textedit);
            result.world.add(model);
        }
        result.world.re_layout();
        result
    }
}

impl ModalWorld for HelpWorld {
    fn get_mut(&mut self) -> &mut dyn World {
        &mut self.world
    }

    fn get(&self) -> &dyn World {
        &self.world
    }

    fn apply_action(
        &mut self,
        _context: &StateContext,
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

    fn add_modal(
        &mut self,
        context: &StateContext,
        chars: &mut HashSet<char>,
        model: Box<dyn Model>,
    ) {
        chars.extend(model.to_string().chars());
        self.world.add_next(model);
        self.world.re_layout();
        let adjustment = if context.global_direction == Direction::Horizontal {
            CameraAdjustment::FitWidth
        } else {
            CameraAdjustment::FitHeight
        };
        self.world.look_next(adjustment);
    }
}
