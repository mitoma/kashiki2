use std::{
    collections::HashSet,
    io,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use chrono::{DateTime, FixedOffset, TimeZone};
use font_rasterizer::{
    glyph_instances::GlyphInstances, glyph_vertex_buffer::Direction,
    vector_instances::VectorInstances,
};
use markdown_heading_splitter::{Heading, leading_heading};
use stroke_parser::{Action, ActionArgument};
use tagbase_memo::{
    Error as TagbaseError, Memo, Result as TagbaseResult, TagbaseMemo, TagbaseState,
};
use text_buffer::action::EditorOperation;
use ui_support::{
    InputResult,
    camera::CameraAdjustment,
    layout_engine::{DefaultWorld, Model, ModelBorder, ModelOperation, World},
    ui::{Card, CardAnchor, SelectBox, SelectOption, TextEdit, TextInput},
    ui_context::UiContext,
};

use super::ModalWorld;
use crate::kashikishi_actions::{insert_date_select, open_file_ui};

pub(crate) struct TagbaseMemosWorld {
    world: DefaultWorld,
    repository: Option<TagbaseMemo>,
    memos: Vec<Memo>,
    state: TagbaseState,
    error_message: Option<String>,
    meta_card: Card,
    // meta_card に表示中のテキスト。変化したときだけ書き換える
    meta_text: Option<String>,
}

impl TagbaseMemosWorld {
    pub(crate) fn new(context: &UiContext) -> Self {
        let mut meta_card = Card::with_settings(context.editor_settings());
        meta_card.set_border(ModelBorder::Rounded);
        let mut result = Self {
            world: DefaultWorld::new(context.window_size()),
            repository: None,
            memos: Vec::new(),
            state: TagbaseState::default(),
            error_message: None,
            meta_card,
            meta_text: None,
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
                if content.title.as_deref().is_none_or(str::is_empty)
                    && let Some(Heading::H1(heading)) = leading_heading(&text)
                    && !heading.is_empty()
                {
                    content.title = Some(heading);
                }
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
        let index = self.world.focus();
        (index < self.memos.len()).then_some(index)
    }

    fn current_meta_text(&self) -> String {
        if self.error_message.is_some() {
            return String::new();
        }
        let Some(index) = self.focused_memo_index() else {
            return String::new();
        };
        let memo = &self.memos[index];
        let title = memo.content.title.as_deref().unwrap_or("（無題）");
        let tags = if memo.content.tags.is_empty() {
            "なし".to_string()
        } else {
            memo.content.tags.join(", ")
        };
        let updated = memo.updated_at.format("%Y-%m-%d %H:%M");
        format!("{title}\nタグ: {tags}\n更新: {updated}")
    }

    // フォーカス移動は World 側で直接処理されるため、毎フレーム差分を確認して反映する
    fn update_meta_card(&mut self, context: &UiContext) {
        let text = self.current_meta_text();
        if self.meta_text.as_ref() != Some(&text) {
            context.register_string(text.clone());
            self.meta_card.set_text(context, text.clone());
            self.meta_text = Some(text);
        }
        // 縦書きでは本文が右から並ぶため、カードは左下に置く
        self.meta_card.set_anchor(match context.global_direction() {
            Direction::Horizontal => CardAnchor::BottomRight,
            Direction::Vertical => CardAnchor::BottomLeft,
        });
        self.meta_card.update(context);
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
                Action::new_command("tagbase-memo", "set-title"),
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
                Action::new_command("tagbase-memo", "set-tags"),
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
            Action::new_command("tagbase-memo", "delete-memo"),
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

    fn close_focused(&mut self, context: &UiContext) {
        let Some(index) = self.focused_memo_index() else {
            return;
        };
        if let Err(error) = self.save_visible() {
            self.display_error(context, error);
            return;
        }
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

    fn open_memo_ui(&mut self, context: &UiContext, chars: &mut HashSet<char>) {
        let Some(repository) = self.repository.as_ref() else {
            self.display_error(context, "タグメモ保管庫を開けません");
            return;
        };
        let stored = match repository.list_memos_with_order(self.state.sort_order) {
            Ok(memos) => memos,
            Err(error) => {
                self.display_error(context, error);
                return;
            }
        };
        let texts = self.world.strings();
        let options: Vec<SelectOption> =
            open_memo_options(&self.memos, &texts, self.focused_memo_index(), &stored)
                .into_iter()
                .map(|(label, id)| {
                    SelectOption::new(
                        label,
                        Action::new_command_with_argument("tagbase-memo", "open-memo", &id),
                    )
                })
                .collect();
        if options.is_empty() {
            self.display_error(context, "開けるメモがありません");
            return;
        }
        self.add_modal(
            context,
            chars,
            Box::new(SelectBox::new_without_action_name(
                context,
                "開くメモを選択してください".to_string(),
                options,
                None,
            )),
        );
    }

    fn open_memo(&mut self, context: &UiContext, id: &str) {
        let id = PathBuf::from(id);
        let in_sync = self.error_message.is_none() && self.memos.len() == self.world.model_length();
        if let Some(index) = self.memos.iter().position(|memo| memo.id == id) {
            if in_sync {
                if let Err(error) = self.world.switch_model(index) {
                    self.display_error(context, error);
                }
            } else {
                self.error_message = None;
                self.reset_world(context);
                self.world.look_at(index, CameraAdjustment::NoCare);
            }
            return;
        }
        let result = self
            .repository
            .as_ref()
            .expect("repository exists")
            .get_memo(&id);
        let memo = match result {
            Ok(memo) => memo,
            Err(error) => {
                self.display_error(context, error);
                return;
            }
        };
        let index = self.memos.len();
        if in_sync {
            // 分割表示などの状態を保つため、作り直さずに文書を追加してから切り替える
            self.world
                .add(Box::new(text_edit(context, &memo.content.text)));
            self.memos.push(memo);
            self.world.re_layout();
            if let Err(error) = self.world.switch_model(index) {
                self.display_error(context, error);
            }
        } else {
            self.memos.push(memo);
            self.error_message = None;
            self.reset_world(context);
            self.world.look_at(index, CameraAdjustment::NoCare);
        }
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
    fn command_namespace(&self) -> Option<&'static str> {
        Some("tagbase-memo")
    }

    fn get_mut(&mut self) -> &mut dyn World {
        &mut self.world
    }

    fn get(&self) -> &dyn World {
        &self.world
    }

    fn update(&mut self, context: &UiContext) {
        self.world.update(context);
        self.update_meta_card(context);
    }

    fn overlay_glyph_instances(&self) -> Vec<&GlyphInstances> {
        self.meta_card.get_instances()
    }

    fn overlay_vector_instances(&self) -> Vec<&VectorInstances<String>> {
        // 表示する情報がないときに空の枠だけが出ないようにする
        if self.meta_text.as_deref().is_none_or(str::is_empty) {
            return Vec::new();
        }
        self.meta_card.get_vector_instances()
    }

    fn overlay_surface_instances(&self) -> Vec<&VectorInstances<String>> {
        if self.meta_text.as_deref().is_none_or(str::is_empty) {
            return Vec::new();
        }
        self.meta_card.get_surface_instances()
    }

    fn world_chars(&self) -> HashSet<char> {
        self.world.chars()
    }

    // World だけでモデルを並べ替えたり消したりすると、表示順で対応づけている memos とずれて
    // メタ情報の表示や保存先を取り違えるため、同じ操作を memos にも反映する
    fn before_world_action(&mut self, context: &UiContext, name: &str) -> bool {
        if self.memos.len() != self.world.model_length() {
            return false;
        }
        let focus = self.world.focus();
        match name {
            "swap-next" if focus + 1 < self.memos.len() => self.memos.swap(focus, focus + 1),
            "swap-prev" if focus > 0 && focus < self.memos.len() => {
                self.memos.swap(focus - 1, focus)
            }
            // モーダルを閉じる用途ではなく、メモ自体を外す場合は保存してから閉じる
            "remove-current" if self.world.modal_instances().0.is_empty() => {
                self.close_focused(context);
                context.register_string(self.world_chars().into_iter().collect());
                return true;
            }
            _ => {}
        }
        false
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
        if *namespace != "tagbase-memo"
            && !(*namespace == "kashikishi"
                && matches!(name.as_str(), "save" | "insert-date" | "open-file-ui"))
        {
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
            "insert-date" => {
                self.add_modal(context, &mut chars, Box::new(insert_date_select(context)))
            }
            "open-file-ui" => self.add_modal(context, &mut chars, Box::new(open_file_ui(context))),
            "add-memo" => self.add_memo(context),
            "open-memo-ui" => self.open_memo_ui(context, &mut chars),
            "open-memo" => {
                if let ActionArgument::String(id) = argument {
                    self.open_memo(context, &id);
                    chars.extend(self.world_chars());
                }
            }
            "search-ui" => self.add_modal(
                context,
                &mut chars,
                Box::new(TextInput::new(
                    context,
                    "検索クエリ（空欄で既定タグ）".to_string(),
                    self.state.search_query.clone(),
                    Action::new_command("tagbase-memo", "search"),
                )),
            ),
            "search" => {
                if let ActionArgument::String2(query, _) = argument {
                    self.search(context, query);
                    chars.extend(self.world_chars());
                }
            }
            "list-all" => {
                self.refresh_all(context);
                chars.extend(self.world_chars());
            }
            "edit-title-ui" => self.edit_title_ui(context, &mut chars),
            "edit-tags-ui" => self.edit_tags_ui(context, &mut chars),
            "set-title" => {
                if let ActionArgument::String2(value, _) = argument {
                    self.set_title(context, value);
                    chars.extend(self.world_chars());
                }
            }
            "set-tags" => {
                if let ActionArgument::String2(value, _) = argument {
                    self.set_tags(context, value);
                    chars.extend(self.world_chars());
                }
            }
            "delete-memo-ui" => self.delete_memo_ui(context, &mut chars),
            "delete-memo" => {
                self.delete_focused(context);
                chars.extend(self.world_chars());
            }
            "close-memo" => {
                self.close_focused(context);
                chars.extend(self.world_chars());
            }
            _ => return (InputResult::Noop, chars),
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

fn memo_label(title: Option<&str>, text: &str) -> String {
    let label = title
        .filter(|title| !title.trim().is_empty())
        .or_else(|| text.lines().find(|line| !line.trim().is_empty()))
        .unwrap_or("（空のメモ）");
    label.chars().take(80).collect()
}

/// メモを開くダイアログの選択肢（表示名とメモ ID）。
/// world:switch-model と同様に表示中のメモを番号付きで並べ、続けて表示していないメモを並べる。
fn open_memo_options(
    opened: &[Memo],
    opened_texts: &[String],
    focus: Option<usize>,
    stored: &[Memo],
) -> Vec<(String, String)> {
    let opened_options = opened
        .iter()
        .enumerate()
        .filter(|(index, _)| Some(*index) != focus)
        .map(|(index, memo)| {
            // 表示中のメモは保存前の編集内容も反映する
            let text = opened_texts
                .get(index)
                .map_or(memo.content.text.as_str(), String::as_str);
            let label = memo_label(memo.content.title.as_deref(), text);
            (
                format!("{}: {label}", index + 1),
                memo.id.to_string_lossy().into_owned(),
            )
        });
    let stored_options = stored
        .iter()
        .filter(|memo| opened.iter().all(|opened| opened.id != memo.id))
        .map(|memo| {
            let label = memo_label(memo.content.title.as_deref(), &memo.content.text);
            (
                format!("未表示: {label}"),
                memo.id.to_string_lossy().into_owned(),
            )
        });
    opened_options.chain(stored_options).collect()
}

static FIXED_CLOCK_BASE: OnceLock<DateTime<FixedOffset>> = OnceLock::new();

/// E2E 用。メモの日時を、固定の基準時刻にテスト用時計の経過時間を足したものにする
pub(crate) fn use_fixed_tagbase_clock() {
    let base = FixedOffset::east_opt(9 * 3600)
        .and_then(|offset| offset.with_ymd_and_hms(2026, 1, 1, 9, 0, 0).single())
        .expect("valid fixed datetime");
    let _ = FIXED_CLOCK_BASE.set(base);
}

fn open_repository() -> TagbaseResult<TagbaseMemo> {
    let repository = TagbaseMemo::new(tagbase_memos_root()?)?;
    Ok(match FIXED_CLOCK_BASE.get().copied() {
        Some(base) => repository.with_clock(move || {
            base + chrono::Duration::milliseconds(font_rasterizer::time::now_millis() as i64)
        }),
        None => repository,
    })
}

fn tagbase_memos_root() -> TagbaseResult<PathBuf> {
    crate::paths::tagbase_memos_dir().ok_or_else(|| TagbaseError::Io {
        path: Path::new("~").join("kashikishi-docs"),
        source: io::Error::new(
            io::ErrorKind::NotFound,
            "ユーザーのホームディレクトリを取得できません",
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tagbase_memo::MemoContent;

    fn memo(id: &str, title: Option<&str>, text: &str) -> Memo {
        let time = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
            .unwrap();
        Memo {
            id: PathBuf::from(id),
            created_at: time,
            updated_at: time,
            content: MemoContent {
                title: title.map(str::to_owned),
                tags: Vec::new(),
                text: text.to_owned(),
            },
        }
    }

    #[test]
    fn open_memo_options_list_opened_memos_then_unopened_memos() {
        let opened = vec![memo("a.md", None, "a"), memo("b.md", Some("B"), "b")];
        let texts = vec!["\n編集中の a".to_owned(), "b".to_owned()];
        let stored = vec![
            memo("c.md", None, "\n\nc の本文"),
            memo("a.md", None, "a"),
            memo("d.md", Some(""), ""),
            memo("e.md", Some("E"), "e の本文"),
        ];
        assert_eq!(
            open_memo_options(&opened, &texts, Some(1), &stored),
            vec![
                ("1: 編集中の a".to_owned(), "a.md".to_owned()),
                ("未表示: c の本文".to_owned(), "c.md".to_owned()),
                ("未表示: （空のメモ）".to_owned(), "d.md".to_owned()),
                ("未表示: E".to_owned(), "e.md".to_owned()),
            ]
        );
    }
}
