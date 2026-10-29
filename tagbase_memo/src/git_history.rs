use std::path::Path;

use git2::{
    ErrorCode, IndexAddOption, Repository, RepositoryInitOptions, RepositoryOpenFlags,
    RepositoryState, Signature,
};

use crate::{Error, Result, SETTINGS_FILE, TagbaseMemo};

const STATE_FILE: &str = "kashikishi-state.toml";
const PATHS: [&str; 3] = ["memos", SETTINGS_FILE, ".gitignore"];

pub(super) fn open_repository(root: &Path) -> std::result::Result<Repository, git2::Error> {
    match Repository::open_ext(root, RepositoryOpenFlags::NO_SEARCH, None::<&Path>) {
        Ok(repository) if !repository.is_bare() => Ok(repository),
        Ok(_) => Err(git2::Error::from_str("保管庫には作業ツリーが必要です")),
        Err(error) if error.code() == ErrorCode::NotFound => {
            Repository::init_opts(root, RepositoryInitOptions::new().no_reinit(true))
        }
        Err(error) => Err(error),
    }
}

impl TagbaseMemo {
    /// メモ・共有設定・除外設定の変更をコミットする。変更がなければ `false` を返す。
    pub fn commit(&self) -> Result<bool> {
        self.commit_changes().map_err(|source| Error::Git {
            path: self.root().to_owned(),
            source,
        })
    }

    fn commit_changes(&self) -> std::result::Result<bool, git2::Error> {
        let repository = open_repository(self.root())?;
        let mut index = repository.index()?;
        if repository.state() != RepositoryState::Clean || index.has_conflicts() {
            return Err(git2::Error::from_str(
                "マージや競合の解消後にもう一度保存してください",
            ));
        }
        let parent = match repository.head() {
            Ok(head) => Some(head.peel_to_commit()?),
            Err(error) if matches!(error.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) => {
                None
            }
            Err(error) => return Err(error),
        };
        match &parent {
            Some(parent) => index.read_tree(&parent.tree()?)?,
            None => index.clear()?,
        }
        index.update_all(PATHS, None)?;
        index.add_all(PATHS, IndexAddOption::DEFAULT, None)?;
        if index.get_path(Path::new(STATE_FILE), 0).is_some() {
            index.remove_path(Path::new(STATE_FILE))?;
        }
        let tree_id = index.write_tree()?;
        if parent
            .as_ref()
            .is_some_and(|parent| parent.tree_id() == tree_id)
        {
            return Ok(false);
        }
        let entries: Vec<_> = index
            .iter()
            .filter(|entry| managed_path(&entry.path))
            .collect();
        let signature = repository
            .signature()
            .or_else(|_| Signature::now("kashikishi", "kashikishi@localhost"))?;
        let message = format!("メモを保存 {}", chrono::Local::now().to_rfc3339());
        let tree = repository.find_tree(tree_id)?;
        let parents: Vec<_> = parent.iter().collect();
        repository.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &message,
            &tree,
            &parents,
        )?;
        index.read(true)?;
        index.remove_all(["memos", SETTINGS_FILE, ".gitignore", STATE_FILE], None)?;
        for entry in entries {
            index.add(&entry)?;
        }
        index.write()?;
        Ok(true)
    }
}

fn managed_path(path: &[u8]) -> bool {
    path.starts_with(b"memos/")
        || path == SETTINGS_FILE.as_bytes()
        || path == b".gitignore"
        || path == STATE_FILE.as_bytes()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::TagbaseState;

    #[test]
    fn saves_creation_update_and_deletion_without_empty_commits() {
        let root = tempfile::tempdir().unwrap();
        let memos = TagbaseMemo::new(root.path()).unwrap();
        let memo = memos.create_memo().unwrap();
        TagbaseState::default().save(memos.root()).unwrap();
        assert!(memos.commit().unwrap());
        let git = open_repository(memos.root()).unwrap();
        let initial = git.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(initial.parent_count(), 0);
        let tree = initial.tree().unwrap();
        assert!(tree.get_path(&memo.id).is_ok());
        assert!(tree.get_name(SETTINGS_FILE).is_some());
        assert!(tree.get_name(".gitignore").is_some());
        assert!(tree.get_name(STATE_FILE).is_none());
        assert!(!memos.commit().unwrap());

        memos
            .update_memo(&memo.id, |content| content.text = "updated".to_owned())
            .unwrap();
        assert!(memos.commit().unwrap());
        let updated = git.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(updated.parent_id(0).unwrap(), initial.id());
        let updated_tree = updated.tree().unwrap();
        let entry = updated_tree.get_path(&memo.id).unwrap();
        let blob = git.find_blob(entry.id()).unwrap();
        assert!(
            std::str::from_utf8(blob.content())
                .unwrap()
                .contains("updated")
        );

        memos.delete_memo(&memo.id).unwrap();
        assert!(memos.commit().unwrap());
        let deleted = git.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(deleted.parent_id(0).unwrap(), updated.id());
        assert!(deleted.tree().unwrap().get_path(&memo.id).is_err());
        let mut options = git2::StatusOptions::new();
        options.include_ignored(false);
        let statuses = git.statuses(Some(&mut options)).unwrap();
        assert!(
            statuses.is_empty(),
            "{:?}",
            statuses
                .iter()
                .map(|entry| (entry.path().map(str::to_owned), entry.status()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn keeps_unrelated_staged_changes_out_of_the_commit() {
        let root = tempfile::tempdir().unwrap();
        let memos = TagbaseMemo::new(root.path()).unwrap();
        let git = open_repository(memos.root()).unwrap();
        let unrelated = Path::new("unrelated.txt");
        fs::write(memos.root().join(unrelated), "staged content").unwrap();
        let mut index = git.index().unwrap();
        index.add_path(unrelated).unwrap();
        index.write().unwrap();
        let staged_id = index.get_path(unrelated, 0).unwrap().id;
        fs::write(memos.root().join(unrelated), "unstaged content").unwrap();

        memos.create_memo().unwrap();
        assert!(memos.commit().unwrap());
        let commit = git.head().unwrap().peel_to_commit().unwrap();
        assert!(commit.tree().unwrap().get_path(unrelated).is_err());
        index.read(true).unwrap();
        assert_eq!(index.get_path(unrelated, 0).unwrap().id, staged_id);
        assert_eq!(
            fs::read_to_string(memos.root().join(unrelated)).unwrap(),
            "unstaged content"
        );
    }

    #[test]
    fn creates_a_vault_repository_instead_of_using_a_parent_repository() {
        let root = tempfile::tempdir().unwrap();
        Repository::init(root.path()).unwrap();
        let memos = TagbaseMemo::new(root.path().join("vault")).unwrap();
        assert!(memos.root().join(".git").is_dir());
        memos.create_memo().unwrap();
        assert!(memos.commit().unwrap());
        assert!(Repository::open(root.path()).unwrap().head().is_err());
    }

    #[test]
    fn uses_configured_author_and_reopens_existing_history() {
        let root = tempfile::tempdir().unwrap();
        let memos = TagbaseMemo::new(root.path()).unwrap();
        let git = open_repository(memos.root()).unwrap();
        let mut config = git.config().unwrap();
        config.set_str("user.name", "Memo author").unwrap();
        config.set_str("user.email", "memo@example.com").unwrap();
        assert!(memos.commit().unwrap());
        let initial = git.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(initial.author().name(), Some("Memo author"));
        let reopened = TagbaseMemo::new(memos.root()).unwrap();
        assert!(!reopened.commit().unwrap());
        assert_eq!(git.head().unwrap().target(), Some(initial.id()));
    }

    #[test]
    fn falls_back_to_application_author_without_valid_identity() {
        let root = tempfile::tempdir().unwrap();
        let memos = TagbaseMemo::new(root.path()).unwrap();
        let git = open_repository(memos.root()).unwrap();
        let mut config = git.config().unwrap();
        config.set_str("user.name", "").unwrap();
        config.set_str("user.email", "").unwrap();
        assert!(memos.commit().unwrap());
        let commit = git.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(commit.author().name(), Some("kashikishi"));
        assert_eq!(commit.author().email(), Some("kashikishi@localhost"));
    }

    #[test]
    fn refuses_to_commit_during_a_merge_without_changing_the_index() {
        let root = tempfile::tempdir().unwrap();
        let memos = TagbaseMemo::new(root.path()).unwrap();
        assert!(memos.commit().unwrap());
        let git = open_repository(memos.root()).unwrap();
        let head = git.head().unwrap().target().unwrap();
        fs::write(git.path().join("MERGE_HEAD"), format!("{head}\n")).unwrap();
        let index = fs::read(git.path().join("index")).unwrap();
        memos.create_memo().unwrap();
        assert!(matches!(memos.commit(), Err(Error::Git { .. })));
        assert_eq!(git.head().unwrap().target(), Some(head));
        assert_eq!(fs::read(git.path().join("index")).unwrap(), index);
    }

    #[test]
    fn removes_previously_tracked_state_without_deleting_the_local_file() {
        let root = tempfile::tempdir().unwrap();
        let memos = TagbaseMemo::new(root.path()).unwrap();
        let git = open_repository(memos.root()).unwrap();
        fs::write(memos.root().join(STATE_FILE), "sort_order = 'desc'\n").unwrap();
        let mut index = git.index().unwrap();
        index.add_path(Path::new(STATE_FILE)).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = git.find_tree(tree_id).unwrap();
        let signature = Signature::now("Test", "test@example.com").unwrap();
        git.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "existing state",
            &tree,
            &[],
        )
        .unwrap();

        TagbaseState::default().save(memos.root()).unwrap();
        assert!(memos.commit().unwrap());
        let commit = git.head().unwrap().peel_to_commit().unwrap();
        assert!(commit.tree().unwrap().get_name(STATE_FILE).is_none());
        assert!(memos.root().join(STATE_FILE).exists());
        index.read(true).unwrap();
        assert!(index.get_path(Path::new(STATE_FILE), 0).is_none());
        assert!(!memos.commit().unwrap());
    }
}
