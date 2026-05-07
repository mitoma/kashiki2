use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use markdown_heading_splitter::split_headings;
use stroke_parser::Action;
use ui_support::{
    camera::CameraAdjustment,
    layout_engine::{DefaultWorld, Model, World},
    ui::FileChooser,
    ui_context::UiContext,
};

use crate::world::ModalWorld;

pub(crate) struct MarkdownPresentationWorld {
    world: DefaultWorld,
    markdown_path: Option<PathBuf>,
}

impl MarkdownPresentationWorld {
    pub(crate) fn new(context: &UiContext) -> Self {
        let world = {
            let mut world = DefaultWorld::new(context.window_size());
            let file_chooser = FileChooser::new(
                context,
                Path::new("."),
                "Markdown ファイルを選択してください",
                stroke_parser::Action::new_command("kashikishi", "open-markdown"),
            );
            context.register_string(file_chooser.to_string());
            world.add_modal(Box::new(file_chooser));
            world.re_layout();
            world
        };

        let _ = context
            .post_action_sender()
            .send(Action::new_command("world", "look-current-and-centering"));

        Self {
            world,
            markdown_path: None,
        }
    }

    pub(crate) fn open_markdown(&mut self, context: &UiContext, path: PathBuf) {
        self.markdown_path = Some(path);

        if let Some(markdown_path) = &self.markdown_path {
            let markdown_content = std::fs::read_to_string(markdown_path).unwrap_or_else(|_| {
                format!(
                    "Failed to read the markdown file: {}",
                    markdown_path.display()
                )
            });

            let markdowns = split_headings(&markdown_content);

            for (heading, content) in markdowns {
                let mut textedit = ui_support::ui::TextEdit::from_context(context);
                textedit.editor_operation(&text_buffer::action::EditorOperation::InsertString(
                    format!(
                        "{} {}\n\n{}",
                        "#".repeat(heading.level()),
                        heading.title(),
                        content
                    ),
                ));
                textedit.editor_operation(&text_buffer::action::EditorOperation::BufferHead);
                textedit.model_operation(
                    &ui_support::layout_engine::ModelOperation::SetHighlightMode(
                        ui_support::ui_context::HighlightMode::Markdown,
                    ),
                );
                textedit.model_operation(
                    &ui_support::layout_engine::ModelOperation::SetModelBorder(
                        ui_support::layout_engine::ModelBorder::Rounded,
                    ),
                );
                let model = Box::new(textedit);
                self.world.add(model);
            }
            self.world.re_layout();
            self.world.look_modal(CameraAdjustment::FitBoth);
        }
    }
}

impl ModalWorld for MarkdownPresentationWorld {
    fn get_mut(&mut self) -> &mut dyn World {
        &mut self.world
    }

    fn get(&self) -> &dyn World {
        &self.world
    }

    fn world_chars(&self) -> HashSet<char> {
        self.world.chars()
    }

    fn add_modal(
        &mut self,
        _context: &UiContext,
        chars: &mut HashSet<char>,
        model: Box<dyn Model>,
    ) {
        chars.extend(model.to_string().chars());
        self.world.add_modal(model);
        self.world.re_layout();
        self.world.look_modal(CameraAdjustment::FitBoth);
    }

    fn apply_action(
        &mut self,
        context: &ui_support::ui_context::UiContext,
        action: stroke_parser::Action,
    ) -> (ui_support::InputResult, std::collections::HashSet<char>) {
        match action {
            stroke_parser::Action::Command(namespace, name, argument)
                if namespace == "kashikishi".into() && name == "open-markdown".into() =>
            {
                if let stroke_parser::ActionArgument::String(path) = argument {
                    self.open_markdown(context, PathBuf::from(path));
                }
                (ui_support::InputResult::InputConsumed, self.world_chars())
            }
            _ => (ui_support::InputResult::Noop, HashSet::new()),
        }
    }
}
