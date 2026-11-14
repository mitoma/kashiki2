use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{Error, Memo, Query, Result, SortOrder, TagbaseMemo, io_at, staged_file};

const STATE_FILE: &str = "kashikishi-state.toml";

/// 保管庫ごとの個人用状態。画面との同期と保存タイミングは利用側で管理する。
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TagbaseState {
    pub narrowing_tag: Vec<String>,
    #[serde(with = "StoredSortOrder")]
    pub sort_order: SortOrder,
    pub open_files: Vec<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focused_file: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "SortOrder", rename_all = "lowercase")]
enum StoredSortOrder {
    Asc,
    Desc,
}

impl TagbaseState {
    /// 保管庫直下の状態ファイルを読む。存在しない場合は `None` を返す。
    pub fn load(root: &Path) -> Result<Option<Self>> {
        let path = root.join(STATE_FILE);
        match fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text)
                .map(Some)
                .map_err(|error| Error::Format {
                    path,
                    message: error.to_string(),
                }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(Error::Io { path, source }),
        }
    }

    /// 状態を一時ファイル経由で保存し、保管庫の `.gitignore` に追加する。
    pub fn save(&self, root: &Path) -> Result<()> {
        let path = root.join(STATE_FILE);
        let text = toml::to_string_pretty(self).map_err(|error| Error::Format {
            path: path.clone(),
            message: error.to_string(),
        })?;
        let ignore_path = root.join(".gitignore");
        let mut ignore = match fs::read_to_string(&ignore_path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
            Err(source) => {
                return Err(Error::Io {
                    path: ignore_path,
                    source,
                });
            }
        };
        if !ignore.lines().any(|line| {
            let line = line.trim();
            line == STATE_FILE || line == format!("/{STATE_FILE}")
        }) {
            if !ignore.is_empty() && !ignore.ends_with('\n') {
                ignore.push('\n');
            }
            ignore.push_str(&format!("/{STATE_FILE}\n"));
            write_atomic(&ignore_path, &ignore)?;
        }
        write_atomic(&path, &text)
    }

    /// 保存済みの順序でメモを取得する。削除済みのメモは除外する。
    pub fn restore_memos(&self, repository: &TagbaseMemo) -> Result<Vec<Memo>> {
        let mut memos = Vec::new();
        for id in &self.open_files {
            match repository.get_memo(id) {
                Ok(memo) => memos.push(memo),
                Err(Error::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
                    log::warn!("保存済みのメモが見つかりません: {}", id.display());
                }
                Err(error) => return Err(error),
            }
        }
        Ok(memos)
    }

    /// 検索クエリと、そのクエリが指定するタグ条件を更新する。
    pub fn set_query(&mut self, input: String, default_tag: &str) -> Result<()> {
        let query = Query::parse(&input)?;
        self.narrowing_tag = if query.tags.is_empty() && query.keywords.is_empty() {
            vec![default_tag.to_owned()]
        } else {
            query.tags
        };
        self.search_query = Some(input);
        Ok(())
    }
}

fn write_atomic(path: &Path, text: &str) -> Result<()> {
    let file = staged_file(path, text)?;
    io_at(path, file.persist(path).map_err(|error| error.error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_round_trip_preserves_order_filter_and_focus() {
        let root = tempfile::tempdir().unwrap();
        let repository = TagbaseMemo::new(root.path()).unwrap();
        let first = repository.create_memo().unwrap();
        let second = repository.create_memo().unwrap();
        let mut state = TagbaseState {
            sort_order: SortOrder::Asc,
            open_files: vec![second.id.clone(), first.id.clone()],
            focused_file: Some(first.id.clone()),
            ..TagbaseState::default()
        };
        state
            .set_query(r#"keyword tag:"my tag" tag:archive"#.to_owned(), "scratch")
            .unwrap();
        state.save(root.path()).unwrap();
        let restored = TagbaseState::load(root.path()).unwrap().unwrap();
        assert_eq!(restored.sort_order, SortOrder::Asc);
        assert_eq!(restored.narrowing_tag, ["my tag", "archive"]);
        assert_eq!(restored.search_query, state.search_query);
        assert_eq!(restored.focused_file, Some(first.id.clone()));
        assert_eq!(
            restored.restore_memos(&repository).unwrap(),
            [second, first]
        );
    }

    #[test]
    fn missing_state_and_empty_open_files_are_distinct() {
        let root = tempfile::tempdir().unwrap();
        let repository = TagbaseMemo::new(root.path()).unwrap();
        repository.create_memo().unwrap();
        assert!(TagbaseState::load(root.path()).unwrap().is_none());
        TagbaseState::default().save(root.path()).unwrap();
        let restored = TagbaseState::load(root.path()).unwrap().unwrap();
        assert!(restored.restore_memos(&repository).unwrap().is_empty());
    }

    #[test]
    fn deleted_files_are_skipped_but_corrupt_files_are_errors() {
        let root = tempfile::tempdir().unwrap();
        let repository = TagbaseMemo::new(root.path()).unwrap();
        let deleted = repository.create_memo().unwrap();
        let existing = repository.create_memo().unwrap();
        let state = TagbaseState {
            open_files: vec![deleted.id.clone(), existing.id.clone()],
            ..TagbaseState::default()
        };
        repository.delete_memo(&deleted.id).unwrap();
        assert_eq!(
            state.restore_memos(&repository).unwrap(),
            std::slice::from_ref(&existing)
        );
        fs::write(root.path().join(existing.id), "invalid front matter").unwrap();
        assert!(state.restore_memos(&repository).is_err());
    }

    #[test]
    fn invalid_state_reports_its_path_and_is_not_rewritten() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(STATE_FILE);
        fs::write(&path, "sort_order = 'invalid'").unwrap();
        let error = TagbaseState::load(root.path()).unwrap_err();
        assert!(matches!(error, Error::Format { path: error_path, .. } if error_path == path));
        assert_eq!(fs::read_to_string(path).unwrap(), "sort_order = 'invalid'");
    }

    #[test]
    fn ignore_file_is_preserved_and_state_entry_is_not_duplicated() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(".gitignore");
        fs::write(&path, "# existing\n*.tmp").unwrap();
        let state = TagbaseState::default();
        state.save(root.path()).unwrap();
        state.save(root.path()).unwrap();
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "# existing\n*.tmp\n/kashikishi-state.toml\n"
        );
    }

    #[test]
    fn empty_query_uses_default_tag_but_keywords_do_not() {
        let mut state = TagbaseState::default();
        state.set_query("  ".to_owned(), "scratch").unwrap();
        assert_eq!(state.narrowing_tag, ["scratch"]);
        state.set_query("keyword".to_owned(), "scratch").unwrap();
        assert!(state.narrowing_tag.is_empty());
    }
}
