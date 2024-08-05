use std::collections::HashSet;

use font_rasterizer::{
    camera::CameraAdjustment,
    context::{StateContext, WindowSize},
    layout_engine::{HorizontalWorld, Model, World},
    support::InputResult,
    ui::textedit::TextEdit,
};
use stroke_parser::{Action, ActionArgument};
use text_buffer::action::EditorOperation;

use crate::{
    categorized_memos::CategorizedMemos,
    kashikishi_actions::{
        add_category_ui, insert_date_select, move_category_ui, move_memo_ui, open_file_ui,
        remove_category_ui,
    },
    memos::Memos,
};

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
    fn add_modal(&mut self, chars: &mut HashSet<char>, model: Box<dyn Model>);
}

pub(crate) struct CategorizedMemosWorld {
    world: HorizontalWorld,
    memos: CategorizedMemos,
}

impl CategorizedMemosWorld {
    pub(crate) fn new(window_size: WindowSize) -> Self {
        let mut result = Self {
            world: HorizontalWorld::new(window_size),
            memos: CategorizedMemos::load_memos(),
        };
        result.reset_world(window_size);
        result
    }

    // ワールドを今のカテゴリでリセットする
    fn reset_world(&mut self, window_size: WindowSize) {
        let mut world = HorizontalWorld::new(window_size);
        for memo in self.memos.get_current_memos().unwrap().memos.iter() {
            let mut textedit = TextEdit::default();
            textedit.editor_operation(&EditorOperation::InsertString(memo.to_string()));
            textedit.editor_operation(&EditorOperation::BufferHead);
            let model = Box::new(textedit);
            world.add(model);
        }
        let look_at = 0;
        world.look_at(look_at, CameraAdjustment::FitBoth);
        world.re_layout();
        self.world = world;
    }
}

impl ModalWorld for CategorizedMemosWorld {
    fn get_mut(&mut self) -> &mut dyn World {
        &mut self.world
    }

    fn get(&self) -> &dyn World {
        &self.world
    }

    fn world_chars(&self) -> HashSet<char> {
        self.world
            .strings()
            .join("")
            .chars()
            .collect::<HashSet<char>>()
    }

    fn add_modal(&mut self, chars: &mut HashSet<char>, model: Box<dyn Model>) {
        chars.extend(model.to_string().chars());
        self.world.add_next(model);
        self.world.re_layout();
        self.world.look_next(CameraAdjustment::FitBoth);
    }

    fn apply_action(
        &mut self,
        context: &StateContext,
        action: Action,
    ) -> (InputResult, HashSet<char>) {
        let mut chars = HashSet::new();

        let Action::Command(namespace, name, argument) = action else {
            return (InputResult::Noop, chars);
        };
        if *namespace != "kashikishi" {
            return (InputResult::Noop, chars);
        };

        match name.as_str() {
            "save" => {
                self.memos
                    .update_current_memos(Memos::from(&self.world as &dyn World));
                self.memos.save_memos().unwrap();
            }
            "add-memo" => {
                let textedit = TextEdit::default();
                let model = Box::new(textedit);
                self.world.add(model);
                self.world.re_layout();
                self.world
                    .look_at(self.world.model_length() - 1, CameraAdjustment::NoCare);
            }
            "remove-memo" => {
                self.world.remove_current();
                self.world.re_layout();
                self.world.look_prev(CameraAdjustment::NoCare);
            }
            "insert-date" => self.add_modal(&mut chars, Box::new(insert_date_select(context))),
            "move-category-ui" => {
                self.add_modal(&mut chars, Box::new(move_category_ui(context, &self.memos)))
            }
            "move-category" => match argument {
                ActionArgument::String(category) => 'outer: {
                    if self.memos.current_category == category {
                        break 'outer;
                    }
                    self.memos
                        .update_current_memos(Memos::from(&self.world as &dyn World));
                    self.memos.current_category = category;
                    self.reset_world(context.window_size);
                    chars.extend(self.world_chars());
                }
                _ => { /* noop */ }
            },
            "move-memo-ui" => {
                self.add_modal(&mut chars, Box::new(move_memo_ui(context, &self.memos)))
            }
            "move-memo" => 'outer: {
                match argument {
                    ActionArgument::String(category) => {
                        if self.memos.current_category == category {
                            break 'outer;
                        }
                        self.memos
                            .add_memo(Some(&category), self.world.current_string());
                        context
                            .action_queue_sender
                            .send(Action::new_command("world", "remove-current"))
                            .unwrap();
                    }
                    _ => { /* noop */ }
                }
            }
            "add-category-ui" => {
                self.add_modal(&mut chars, Box::new(add_category_ui(context)));
            }
            "add-category" => {
                if let ActionArgument::String(category) = argument {
                    if !self.memos.categories().contains(&category) {
                        self.memos.add_memo(Some(&category), String::new());
                    }
                }
            }
            "remove-category-ui" => {
                self.add_modal(
                    &mut chars,
                    Box::new(remove_category_ui(context, &self.memos)),
                );
            }
            "remove-category" => {
                if let ActionArgument::String(category) = argument {
                    self.memos.remove_category(&category);
                    if self.memos.current_category == category {
                        self.reset_world(context.window_size);
                    }
                }
            }
            "open-file-ui" => {
                let arg = match argument {
                    ActionArgument::String(path) => Some(path),
                    _ => None,
                };
                self.add_modal(&mut chars, Box::new(open_file_ui(context, arg.as_deref())));
            }
            _ => { /* noop */ }
        }
        (InputResult::InputConsumed, chars)
    }

    fn graceful_exit(&mut self) {
        self.memos
            .update_current_memos(Memos::from(&self.world as &dyn World));
        self.memos.save_memos().unwrap();
    }
}

pub(crate) struct HelpWorld {
    world: HorizontalWorld,
}

impl HelpWorld {
    pub(crate) fn new(window_size: WindowSize) -> Self {
        let mut result = Self {
            world: HorizontalWorld::new(window_size),
        };

        let help_contents: Vec<&str> = serde_json::from_str(include_str!("help.json")).unwrap();

        for help_content in help_contents {
            let mut textedit = TextEdit::default();
            textedit.editor_operation(&EditorOperation::InsertString(help_content.to_string()));
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
        self.world
            .strings()
            .join("")
            .chars()
            .collect::<HashSet<char>>()
    }

    fn graceful_exit(&mut self) {
        // noop
    }

    fn add_modal(&mut self, chars: &mut HashSet<char>, model: Box<dyn Model>) {
        chars.extend(model.to_string().chars());
        self.world.add_next(model);
        self.world.re_layout();
        self.world.look_next(CameraAdjustment::FitBoth);
    }
}
