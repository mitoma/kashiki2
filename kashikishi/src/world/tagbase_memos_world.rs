use std::{
    collections::HashSet,
    io,
    path::{Path, PathBuf},
};

use font_rasterizer::glyph_vertex_buffer::Direction;
use stroke_parser::{Action, ActionArgument};
use tagbase_memo::{
    Error as TagbaseError, Memo, Result as TagbaseResult, TagbaseMemo, TagbaseState,
};
use text_buffer::action::EditorOperation;
use ui_support::{
    InputResult,
    camera::CameraAdjustment,
    layout_engine::{DefaultWorld, Model, ModelOperation, World},
    ui::{SelectBox, SelectOption, TextEdit, TextInput},
    ui_context::UiContext,
};

use super::ModalWorld;

pub(crate) struct TagbaseMemosWorld {
    world: DefaultWorld,
    repository: Option<TagbaseMemo>,
    memos: Vec<Memo>,
    state: TagbaseState,
    error_message: Option<String>,
}

impl TagbaseMemosWorld {
    pub(crate) fn new(context: &UiContext) -> Self {
        let mut result = Self {
            world: DefaultWorld::new(context.window_size()),
            repository: None,
            memos: Vec::new(),
            state: TagbaseState::default(),
            error_message: None,
        };
        match open_repository().and_then(|repository| {
            let state = TagbaseState::load(repository.root())?;
            let memos = match &state {
                Some(state) => state.restore_memos(&repository)?,
                None => repository.list_memos()?,
            };
            Ok((repository, memos, state.unwrap_or_default()))
        }) {
            Ok((repository, memos, state)) => {
                result.repository = Some(repository);
                result.memos = memos;
                result.state = state;
            }
            Err(error) => result.error_message = Some(error.to_string()),
        }
        result.reset_world(context);
        result
    }

    fn reset_world(&mut self, context: &UiContext) {
        let mut world = DefaultWorld::new(context.window_size());
        if let Some(message) = &self.error_message {
            world.add(Box::new(text_edit(context, message)));
        } else {
            for memo in &self.memos {
                world.add(Box::new(text_edit(context, &memo.content.text)));
            }
        }
        if world.model_length() > 0 {
            let adjustment = match context.global_direction() {
                Direction::Horizontal => CameraAdjustment::FitWidth,
                Direction::Vertical => CameraAdjustment::FitHeight,
            };
            let focus = self
                .memos
                .iter()
                .position(|memo| Some(&memo.id) == self.state.focused_file.as_ref())
                .unwrap_or(0);
            world.look_at(focus, adjustment);
        }
        world.re_layout();
        self.world = world;
    }

    fn save_visible(&mut self) -> TagbaseResult<()> {
        let repository = self.repository.as_ref().ok_or_else(|| TagbaseError::Io {
            path: PathBuf::from("~/kashikishi-docs"),
            source: io::Error::other("タグメモ保管庫を開けません"),
        })?;
        let texts = self.world.strings();
        if texts.len() != self.memos.len() {
            return Err(TagbaseError::Io {
                path: repository.root().to_owned(),
                source: io::Error::other("表示中のメモと保管庫の内容が一致しません"),
            });
        }
        for (memo, text) in self.memos.iter_mut().zip(texts) {
            let updated = repository.update_memo(&memo.id, |content| {
                content.text = text;
            })?;
            *memo = updated;
        }
        Ok(())
    }

    fn save(&mut self) -> TagbaseResult<()> {
        self.save_visible()?;
        self.state.open_files = self.memos.iter().map(|memo| memo.id.clone()).collect();
        self.state.focused_file = self
            .focused_memo_index()
            .map(|index| self.memos[index].id.clone());
        let repository = self
            .repository
            .as_ref()
            .expect("repository exists after saving memos");
        self.state.save(repository.root())?;
        repository.commit()?;
        Ok(())
    }

    fn display_error(&mut self, context: &UiContext, error: impl std::fmt::Display) {
        let message = format!("タグメモの操作に失敗しました\n{error}");
        self.error_message = Some(message.clone());
        self.world.add_modal(Box::new(text_edit(context, &message)));
        self.world.re_layout();
        self.world.look_modal(CameraAdjustment::FitBoth);
    }

    fn display_memos(&mut self, context: &UiContext, result: TagbaseResult<Vec<Memo>>) {
        match result {
            Ok(memos) => {
                self.error_message = None;
                self.memos = memos;
                self.reset_world(context);
            }
            Err(error) => self.display_error(context, error),
        }
    }

    fn focused_memo_index(&self) -> Option<usize> {
        let index = self.world.debug_snapshot().focus;
        (index < self.memos.len()).then_some(index)
    }

    fn edit_title_ui(&mut self, context: &UiContext, chars: &mut HashSet<char>) {
        let Some(index) = self.focused_memo_index() else {
            self.display_error(context, "タイトルを編集するメモがありません");
            return;
        };
        if let Err(error) = self.save_visible() {
            self.display_error(context, error);
            return;
        }
        let title = self.memos[index].content.title.clone().unwrap_or_default();
        self.add_modal(
            context,
            chars,
            Box::new(TextInput::new(
                context,
                "メモのタイトルを入力（空欄で無題）".to_string(),
                Some(title),
                Action::new_command("kashikishi", "tagbase-set-title"),
            )),
        );
    }

    fn edit_tags_ui(&mut self, context: &UiContext, chars: &mut HashSet<char>) {
        let Some(index) = self.focused_memo_index() else {
            self.display_error(context, "タグを編集するメモがありません");
            return;
        };
        if let Err(error) = self.save_visible() {
            self.display_error(context, error);
            return;
        }
        let tags = self.memos[index].content.tags.join(", ");
        self.add_modal(
            context,
            chars,
            Box::new(TextInput::new(
                context,
                "タグをカンマ区切りで入力".to_string(),
                Some(tags),
                Action::new_command("kashikishi", "tagbase-set-tags"),
            )),
        );
    }

    fn delete_memo_ui(&mut self, context: &UiContext, chars: &mut HashSet<char>) {
        if self.focused_memo_index().is_none() {
            self.display_error(context, "削除するメモがありません");
            return;
        }
        let options = vec![SelectOption::new(
            "現在のメモを削除する".to_string(),
            Action::new_command("kashikishi", "tagbase-delete-memo"),
        )];
        self.add_modal(
            context,
            chars,
            Box::new(SelectBox::new_without_action_name(
                context,
                "削除を確認してください（キャンセルは Esc）".to_string(),
                options,
                None,
            )),
        );
    }

    fn set_title(&mut self, context: &UiContext, value: String) {
        let Some(index) = self.focused_memo_index() else {
            self.display_error(context, "タイトルを編集するメモがありません");
            return;
        };
        let id = self.memos[index].id.clone();
        let title = (!value.is_empty()).then_some(value);
        let result = self
            .repository
            .as_ref()
            .expect("repository exists")
            .update_memo(&id, |content| content.title = title);
        match result {
            Ok(memo) => {
                self.memos[index] = memo;
                self.error_message = None;
                self.reset_world(context);
                self.world.look_at(index, CameraAdjustment::NoCare);
            }
            Err(error) => self.display_error(context, error),
        }
    }

    fn set_tags(&mut self, context: &UiContext, value: String) {
        let Some(index) = self.focused_memo_index() else {
            self.display_error(context, "タグを編集するメモがありません");
            return;
        };
        let tags = value
            .split(',')
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(str::to_owned)
            .collect();
        let id = self.memos[index].id.clone();
        let result = self
            .repository
            .as_ref()
            .expect("repository exists")
            .update_memo(&id, |content| content.tags = tags);
        match result {
            Ok(memo) => {
                self.memos[index] = memo;
                self.error_message = None;
                self.reset_world(context);
                self.world.look_at(index, CameraAdjustment::NoCare);
            }
            Err(error) => self.display_error(context, error),
        }
    }

    fn delete_focused(&mut self, context: &UiContext) {
        let Some(index) = self.focused_memo_index() else {
            return;
        };
        if let Err(error) = self.save_visible() {
            self.display_error(context, error);
            return;
        }
        let id = self.memos[index].id.clone();
        let result = self
            .repository
            .as_ref()
            .expect("repository exists")
            .delete_memo(&id);
        match result {
            Ok(()) => {
                self.memos.remove(index);
                self.error_message = None;
                self.reset_world(context);
                if self.world.model_length() > 0 {
                    self.world.look_at(
                        index.min(self.world.model_length() - 1),
                        CameraAdjustment::NoCare,
                    );
                }
            }
            Err(error) => self.display_error(context, error),
        }
    }

    fn refresh_all(&mut self, context: &UiContext) {
        if let Err(error) = self.save_visible() {
            self.display_error(context, error);
            return;
        }
        let result = self
            .repository
            .as_ref()
            .expect("repository exists")
            .list_memos_with_order(self.state.sort_order);
        if result.is_ok() {
            self.state.narrowing_tag.clear();
            self.state.search_query = None;
        }
        self.display_memos(context, result);
    }

    fn search(&mut self, context: &UiContext, query: String) {
        if let Err(error) = self.save_visible() {
            self.display_error(context, error);
            return;
        }
        let repository = self.repository.as_ref().expect("repository exists");
        let result = repository
            .search_with_order(&query, self.state.sort_order)
            .and_then(|memos| {
                self.state
                    .set_query(query, &repository.settings().default_tag)?;
                Ok(memos)
            });
        self.display_memos(context, result);
    }

    fn add_memo(&mut self, context: &UiContext) {
        if let Err(error) = self.save_visible() {
            self.display_error(context, error);
            return;
        }
        let result = self
            .repository
            .as_ref()
            .expect("repository exists")
            .create_memo();
        match result {
            Ok(memo) => {
                self.memos.push(memo);
                self.error_message = None;
                self.reset_world(context);
                self.world
                    .look_at(self.memos.len() - 1, CameraAdjustment::NoCare);
            }
            Err(error) => self.display_error(context, error),
        }
    }
}

impl ModalWorld for TagbaseMemosWorld {
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
        context: &UiContext,
        action: Action,
    ) -> (InputResult, HashSet<char>) {
        let mut chars = HashSet::new();
        let Action::Command(namespace, name, argument) = action else {
            return (InputResult::Noop, chars);
        };
        if *namespace != "kashikishi" {
            return (InputResult::Noop, chars);
        }

        match name.as_str() {
            "save" => {
                if let Err(error) = self.save() {
                    self.display_error(context, error);
                } else {
                    self.error_message = None;
                }
            }
            "tagbase-add-memo" => self.add_memo(context),
            "tagbase-search-ui" => self.add_modal(
                context,
                &mut chars,
                Box::new(TextInput::new(
                    context,
                    "検索クエリ（空欄で既定タグ）".to_string(),
                    self.state.search_query.clone(),
                    Action::new_command("kashikishi", "tagbase-search"),
                )),
            ),
            "tagbase-search" => {
                if let ActionArgument::String2(query, _) = argument {
                    self.search(context, query);
                    chars.extend(self.world_chars());
                }
            }
            "tagbase-list-all" => {
                self.refresh_all(context);
                chars.extend(self.world_chars());
            }
            "tagbase-edit-title-ui" => self.edit_title_ui(context, &mut chars),
            "tagbase-edit-tags-ui" => self.edit_tags_ui(context, &mut chars),
            "tagbase-set-title" => {
                if let ActionArgument::String2(value, _) = argument {
                    self.set_title(context, value);
                    chars.extend(self.world_chars());
                }
            }
            "tagbase-set-tags" => {
                if let ActionArgument::String2(value, _) = argument {
                    self.set_tags(context, value);
                    chars.extend(self.world_chars());
                }
            }
            "tagbase-delete-memo-ui" => self.delete_memo_ui(context, &mut chars),
            "tagbase-delete-memo" => {
                self.delete_focused(context);
                chars.extend(self.world_chars());
            }
            _ => {}
        }
        if let Some(message) = &self.error_message {
            chars.extend(message.chars());
        }
        (InputResult::InputConsumed, chars)
    }

    fn graceful_exit(&mut self) {
        if let Err(error) = self.save() {
            log::error!("Failed to save tagbase memos: {error}");
        }
    }
}

fn text_edit(context: &UiContext, text: &str) -> TextEdit {
    let mut textedit = TextEdit::from_context(context);
    textedit.model_operation(&ModelOperation::SetHighlightMode(
        ui_support::ui_context::HighlightMode::Markdown,
    ));
    textedit.editor_operation(&EditorOperation::InsertString(text.to_owned()));
    textedit.editor_operation(&EditorOperation::BufferHead);
    textedit
}

fn open_repository() -> TagbaseResult<TagbaseMemo> {
    TagbaseMemo::new(tagbase_memos_root()?)
}

fn tagbase_memos_root() -> TagbaseResult<PathBuf> {
    dirs::home_dir()
        .map(|home| home.join("kashikishi-docs"))
        .ok_or_else(|| TagbaseError::Io {
            path: Path::new("~").join("kashikishi-docs"),
            source: io::Error::new(
                io::ErrorKind::NotFound,
                "ユーザーのホームディレクトリを取得できません",
            ),
        })
}
