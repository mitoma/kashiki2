use std::collections::HashSet;

use font_rasterizer::{
    context::{StateContext, WindowSize},
    font_buffer::Direction,
};
use stroke_parser::{Action, ActionArgument};
use text_buffer::action::EditorOperation;
use ui_support::{
    camera::CameraAdjustment,
    layout_engine::{DefaultWorld, Model, ModelOperation, World},
    ui::TextEdit,
    InputResult,
};

use crate::{
    categorized_memos::CategorizedMemos,
    kashikishi_actions::{
        add_category_ui, insert_date_select, move_category_ui, move_memo_ui, open_file_ui,
        remove_category_ui, rename_category_select_ui, rename_category_ui,
    },
    memos::Memos,
};

use super::ModalWorld;

pub(crate) struct CategorizedMemosWorld {
    world: DefaultWorld,
    memos: CategorizedMemos,
}

impl CategorizedMemosWorld {
    pub(crate) fn new(window_size: WindowSize, direction: Direction) -> Self {
        let mut result = Self {
            world: DefaultWorld::new(window_size),
            memos: CategorizedMemos::load_memos(),
        };
        result.reset_world(window_size, direction);
        result
    }

    // ワールドを今のカテゴリでリセットする
    fn reset_world(&mut self, window_size: WindowSize, direction: Direction) {
        let mut world = DefaultWorld::new(window_size);
        for memo in self.memos.get_current_memos().unwrap().memos.iter() {
            let mut textedit = TextEdit::default();
            textedit.model_operation(&ModelOperation::ChangeDirection(Some(direction)));
            textedit.editor_operation(&EditorOperation::InsertString(memo.to_string()));
            textedit.editor_operation(&EditorOperation::BufferHead);
            let model = Box::new(textedit);
            world.add(model);
        }
        let look_at = 0;
        let adjustment = match direction {
            Direction::Horizontal => CameraAdjustment::FitWidth,
            Direction::Vertical => CameraAdjustment::FitHeight,
        };
        world.look_at(look_at, adjustment);
        world.re_layout();
        self.world = world;
    }

    fn sync(&mut self) {
        self.memos
            .update_current_memos(Memos::from(&self.world as &dyn World));
    }

    fn save(&mut self) {
        self.memos
            .update_current_memos(Memos::from(&self.world as &dyn World));
        self.memos.save_memos().unwrap();
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
        self.world.chars()
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
            "save" => self.save(),
            "add-memo" => {
                let mut textedit = TextEdit::default();
                textedit.model_operation(&ModelOperation::ChangeDirection(Some(
                    context.global_direction,
                )));
                let model = Box::new(textedit);
                self.world.add(model);
                self.world.re_layout();
                self.world
                    .look_at(self.world.model_length() - 1, CameraAdjustment::NoCare);
                self.sync();
            }
            "remove-memo" => {
                self.world.remove_current();
                self.world.re_layout();
                self.world.look_prev(CameraAdjustment::NoCare);
                self.sync();
            }
            "insert-date" => {
                self.add_modal(context, &mut chars, Box::new(insert_date_select(context)))
            }
            "move-category-ui" => self.add_modal(
                context,
                &mut chars,
                Box::new(move_category_ui(context, &self.memos)),
            ),
            "move-category" => match argument {
                ActionArgument::String(category) => 'outer: {
                    if self.memos.current_category == category {
                        break 'outer;
                    }
                    self.sync();
                    self.memos.current_category = category;
                    self.reset_world(context.window_size, context.global_direction);
                    chars.extend(self.world_chars());
                }
                _ => { /* noop */ }
            },
            "move-memo-ui" => self.add_modal(
                context,
                &mut chars,
                Box::new(move_memo_ui(context, &self.memos)),
            ),
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
                self.add_modal(context, &mut chars, Box::new(add_category_ui(context)));
            }
            "add-category" => {
                if let ActionArgument::String2(category, _) = argument {
                    if !self.memos.categories().contains(&category) {
                        self.memos.add_memo(Some(&category), String::new());
                    }
                }
            }
            "rename-category-select-ui" => {
                self.add_modal(
                    context,
                    &mut chars,
                    Box::new(rename_category_select_ui(context, &self.memos)),
                );
            }
            "rename-category-ui" => {
                if let ActionArgument::String(category) = argument {
                    self.add_modal(
                        context,
                        &mut chars,
                        Box::new(rename_category_ui(context, &category)),
                    );
                }
            }
            "rename-category" => {
                if let ActionArgument::String2(new_name, old_name) = argument {
                    self.memos.rename_category(&new_name, &old_name);
                }
            }
            "remove-category-ui" => {
                self.add_modal(
                    context,
                    &mut chars,
                    Box::new(remove_category_ui(context, &self.memos)),
                );
            }
            "remove-category" => {
                if let ActionArgument::String(category) = argument {
                    self.memos.remove_category(&category);
                    if self.memos.current_category == category {
                        self.reset_world(context.window_size, context.global_direction);
                    }
                }
            }
            "open-file-ui" => {
                let arg = match argument {
                    ActionArgument::String(path) => Some(path),
                    _ => None,
                };
                self.add_modal(
                    context,
                    &mut chars,
                    Box::new(open_file_ui(context, arg.as_deref())),
                );
            }
            _ => { /* noop */ }
        }
        (InputResult::InputConsumed, chars)
    }

    fn graceful_exit(&mut self) {
        self.save();
    }
}
