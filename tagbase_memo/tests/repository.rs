use std::{fs, path::Path};

use chrono::{DateTime, Local};
use tagbase_memo::{Error, Memo, Settings, SortOrder, TagbaseMemo};
use tempfile::{TempDir, tempdir};

fn repository() -> (TempDir, TagbaseMemo) {
    let directory = tempdir().unwrap();
    let repository = TagbaseMemo::new(directory.path()).unwrap();
    (directory, repository)
}

fn create(repository: &TagbaseMemo, title: &str, text: &str, tags: &[&str]) -> Memo {
    let memo = repository.create_memo().unwrap();
    repository
        .update_memo(&memo.id, |content| {
            content.title = Some(title.to_owned());
            content.text = text.to_owned();
            content.tags = tags.iter().map(|tag| (*tag).to_owned()).collect();
        })
        .unwrap()
}

fn ids(memos: Vec<Memo>) -> Vec<std::path::PathBuf> {
    memos.into_iter().map(|memo| memo.id).collect()
}

#[test]
fn initializes_and_reopens_repository_without_touching_user_state() {
    let directory = tempdir().unwrap();
    let root = directory.path().join("new").join("repository");
    let repository = TagbaseMemo::new(&root).unwrap();
    assert_eq!(repository.settings(), &Settings::default());
    assert!(root.join("memos").is_dir());
    assert!(!root.join("kashikishi-state.toml").exists());
    let settings_path = root.join("kashikishi-tagbase-settings.toml");
    let settings = Settings {
        tag_definitions: vec!["custom".to_owned()],
        default_tag: "initial".to_owned(),
        invisible_tag: "hidden".to_owned(),
    };
    fs::write(&settings_path, toml::to_string(&settings).unwrap()).unwrap();
    let reopened = TagbaseMemo::new(&root).unwrap();
    assert_eq!(reopened.settings(), &settings);
    assert_eq!(reopened.create_memo().unwrap().content.tags, ["initial"]);
    assert_eq!(TagbaseMemo::new(&root).unwrap().settings(), &settings);
}

#[test]
fn injected_clock_fixes_timestamps_and_ids() {
    let directory = tempdir().unwrap();
    let fixed = DateTime::parse_from_rfc3339("2026-01-01T09:00:00+09:00").unwrap();
    let repository = TagbaseMemo::new(directory.path())
        .unwrap()
        .with_clock(move || fixed);
    let memo = repository.create_memo().unwrap();
    assert_eq!(memo.created_at, fixed);
    assert_eq!(memo.updated_at, fixed);
    assert_eq!(memo.id, Path::new("memos/2026-01/2026-01-01-001.md"));
}

#[test]
fn creates_local_date_files_with_immutable_metadata() {
    let (_directory, repository) = repository();
    let before = Local::now().fixed_offset();
    let first = repository.create_memo().unwrap();
    let after = Local::now().fixed_offset();
    assert!(before <= first.created_at && first.created_at <= after);
    assert_eq!(first.created_at, first.updated_at);
    assert_eq!(first.content.title, None);
    assert_eq!(first.content.text, "");
    assert_eq!(first.content.tags, ["scratch"]);
    let expected = Path::new("memos")
        .join(first.created_at.format("%Y-%m").to_string())
        .join(format!("{}-001.md", first.created_at.format("%Y-%m-%d")));
    assert_eq!(first.id, expected);
    let path = repository.root().join(&first.id);
    let original = fs::read(&path).unwrap();
    assert!(!String::from_utf8_lossy(&original).contains("title:"));
    let unchanged = repository.update_memo(&first.id, |_| {}).unwrap();
    assert_eq!(unchanged, first);
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(repository.get_memo(&first.id).unwrap(), first);
}

#[test]
fn updates_round_trip_and_do_not_restore_deleted_default_tag() {
    let (directory, repository) = repository();
    let original = repository.create_memo().unwrap();
    let body = "\n日本語\r\n---\n# heading\n\n";
    let updated = repository
        .update_memo(&original.id, |content| {
            content.title = Some("title: \"quoted\"".to_owned());
            content.tags.clear();
            content.text = body.to_owned();
        })
        .unwrap();
    assert_eq!(updated.created_at, original.created_at);
    assert!(updated.updated_at >= original.updated_at);
    assert_eq!(updated.id, original.id);
    assert_eq!(updated.content.text, body);
    let reopened = TagbaseMemo::new(directory.path()).unwrap();
    assert_eq!(reopened.get_memo(&original.id).unwrap(), updated);
    let updated = reopened
        .update_memo(&original.id, |content| content.title = None)
        .unwrap();
    assert!(updated.content.tags.is_empty());
    assert_eq!(reopened.list_memos().unwrap().len(), 1);
    assert!(reopened.search("").unwrap().is_empty());
}

#[test]
fn allocation_uses_maximum_sequence_and_does_not_overwrite_existing_files() {
    let (_directory, repository) = repository();
    let first = repository.create_memo().unwrap();
    let parent = repository.root().join(first.id.parent().unwrap());
    let date = first.created_at.format("%Y-%m-%d");
    let existing = parent.join(format!("{date}-999.md"));
    let bytes = fs::read(repository.root().join(&first.id)).unwrap();
    fs::write(&existing, &bytes).unwrap();
    let next = repository.create_memo().unwrap();
    assert_eq!(
        next.id.file_name().unwrap(),
        format!("{date}-1000.md").as_str()
    );
    assert_eq!(fs::read(&existing).unwrap(), bytes);
    repository.delete_memo(&first.id).unwrap();
    let next = repository.create_memo().unwrap();
    assert_eq!(
        next.id.file_name().unwrap(),
        format!("{date}-1001.md").as_str()
    );
}

#[test]
fn keyword_search_is_case_insensitive_and_does_not_implicitly_filter_default_tag() {
    let (_directory, repository) = repository();
    let first = create(&repository, "Hello 日本語", "other", &["other"]);
    let second = create(&repository, "other", "HELLO 日本語", &["scratch"]);
    create(&repository, "other", "other", &["hello"]);
    let results = repository.search("hello 日本語").unwrap();
    assert_eq!(results.len(), 2);
    assert!(ids(results).contains(&first.id));
    assert_eq!(
        ids(repository.search("tag:scratch HELLO").unwrap()),
        [second.id]
    );
}

#[test]
fn tags_are_exact_and_use_and_conditions_with_unrestricted_names() {
    let (_directory, repository) = repository();
    let memo = create(
        &repository,
        "",
        "",
        &["todo", "custom", "my tag", "tag:example"],
    );
    create(&repository, "", "", &["todo"]);
    assert_eq!(
        ids(repository.search("tag:todo tag:custom").unwrap()),
        std::slice::from_ref(&memo.id)
    );
    assert_eq!(
        ids(repository.filter_by_tags(&["todo", "custom"]).unwrap()),
        std::slice::from_ref(&memo.id)
    );
    assert_eq!(
        ids(repository.search(r#"tag:"my tag""#).unwrap()),
        std::slice::from_ref(&memo.id)
    );
    assert_eq!(
        ids(repository.search(r#"tag:"tag:example""#).unwrap()),
        [memo.id]
    );
    assert!(repository.search("tag:TODO").unwrap().is_empty());
    assert!(repository.search("tag:to").unwrap().is_empty());
}

#[test]
fn quotes_distinguish_literal_keywords_from_tag_conditions() {
    let (_directory, repository) = repository();
    let memo = create(
        &repository,
        "tag:example",
        r#"my phrase "quoted" \path"#,
        &[],
    );
    assert_eq!(
        ids(repository.search(r#""tag:example""#).unwrap()),
        std::slice::from_ref(&memo.id)
    );
    assert_eq!(
        ids(repository.search(r#""my phrase""#).unwrap()),
        std::slice::from_ref(&memo.id)
    );
    assert_eq!(
        ids(repository.search(r#""\"quoted\"""#).unwrap()),
        std::slice::from_ref(&memo.id)
    );
    assert_eq!(ids(repository.search(r#""\\path""#).unwrap()), [memo.id]);
    assert!(repository.search("tag:example").unwrap().is_empty());
}

#[test]
fn empty_search_and_filter_use_default_tag_but_list_does_not() {
    let (_directory, repository) = repository();
    let memo = repository.create_memo().unwrap();
    create(&repository, "", "", &[]);
    for query in ["", "  \t\n"] {
        assert_eq!(
            ids(repository.search(query).unwrap()),
            std::slice::from_ref(&memo.id)
        );
    }
    assert_eq!(ids(repository.filter_by_tags(&[]).unwrap()), [memo.id]);
    assert_eq!(repository.list_memos().unwrap().len(), 2);
}

#[test]
fn hidden_memos_require_explicit_tag_and_still_apply_other_conditions() {
    let (_directory, repository) = repository();
    let hidden = create(&repository, "needle", "", &["archive", "scratch", "todo"]);
    let visible = create(&repository, "needle", "", &["scratch"]);
    assert_eq!(
        ids(repository.list_memos().unwrap()),
        std::slice::from_ref(&visible.id)
    );
    assert_eq!(
        ids(repository.search("needle").unwrap()),
        std::slice::from_ref(&visible.id)
    );
    assert_eq!(ids(repository.search("").unwrap()), [visible.id]);
    assert!(repository.filter_by_tags(&["todo"]).unwrap().is_empty());
    assert_eq!(
        ids(repository.search("needle tag:archive tag:todo").unwrap()),
        std::slice::from_ref(&hidden.id)
    );
    assert_eq!(
        ids(repository.filter_by_tags(&["archive"]).unwrap()),
        std::slice::from_ref(&hidden.id)
    );
    assert!(repository.search("missing tag:archive").unwrap().is_empty());
    assert!(
        repository
            .search("tag:archive tag:missing")
            .unwrap()
            .is_empty()
    );
    assert_eq!(repository.get_memo(&hidden.id).unwrap(), hidden);
}

#[test]
fn malformed_queries_return_explicit_errors() {
    let (_directory, repository) = repository();
    for query in [
        "tag:",
        r#""""#,
        r#"tag:"""#,
        r#""unclosed"#,
        r#""x"tail"#,
        r#"a"b"#,
        r#""\q""#,
    ] {
        assert!(
            matches!(repository.search(query), Err(Error::InvalidQuery(_))),
            "{query}"
        );
    }
}

fn write_external(repository: &TagbaseMemo, id: &str, created: &str, text: &str) {
    let path = repository.root().join("memos").join(id);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        format!(
            "---\ncreated_at: {created}\nupdated_at: {created}\ntags: [scratch]\n---\n\n{text}"
        ),
    )
    .unwrap();
}

#[test]
fn ordering_compares_instants_and_breaks_ties_by_filename_in_both_directions() {
    let (_directory, repository) = repository();
    write_external(&repository, "a.md", "2026-01-01T10:00:00+09:00", "needle");
    write_external(&repository, "b.md", "2026-01-01T01:00:00Z", "needle");
    write_external(&repository, "c.md", "2026-01-01T02:00:00Z", "needle");
    let a = Path::new("memos").join("a.md");
    let b = Path::new("memos").join("b.md");
    let c = Path::new("memos").join("c.md");
    let ascending = vec![a.clone(), b.clone(), c.clone()];
    let descending = vec![c, a, b];
    assert_eq!(ids(repository.list_memos().unwrap()), descending);
    assert_eq!(
        ids(repository.list_memos_with_order(SortOrder::Asc).unwrap()),
        ascending
    );
    assert_eq!(
        ids(repository
            .search_with_order("needle", SortOrder::Asc)
            .unwrap()),
        ascending
    );
    assert_eq!(
        ids(repository
            .filter_by_tags_with_order(&[], SortOrder::Asc)
            .unwrap()),
        ascending
    );
}

#[test]
fn external_rename_and_delete_change_identity_and_missing_targets_are_errors() {
    let (_directory, repository) = repository();
    let memo = repository.create_memo().unwrap();
    let renamed = Path::new("memos").join("renamed.md");
    fs::rename(
        repository.root().join(&memo.id),
        repository.root().join(&renamed),
    )
    .unwrap();
    assert!(matches!(
        repository.get_memo(&memo.id),
        Err(Error::Io { .. })
    ));
    assert_eq!(repository.get_memo(&renamed).unwrap().id, renamed);
    assert!(repository.update_memo(&memo.id, |_| {}).is_err());
    assert!(repository.delete_memo(&memo.id).is_err());
    repository.delete_memo(&renamed).unwrap();
    assert!(repository.list_memos().unwrap().is_empty());
}

#[test]
fn invalid_ids_cannot_read_update_or_delete_outside_memos() {
    let (_directory, repository) = repository();
    for id in [
        "../outside.md",
        "memos/../../outside.md",
        "other/file.md",
        "memos/file.txt",
    ] {
        assert!(
            matches!(repository.get_memo(id), Err(Error::InvalidId(_))),
            "{id}"
        );
        assert!(
            matches!(repository.update_memo(id, |_| {}), Err(Error::InvalidId(_))),
            "{id}"
        );
        assert!(
            matches!(repository.delete_memo(id), Err(Error::InvalidId(_))),
            "{id}"
        );
    }
    let absolute = repository.root().join("memos").join("absolute.md");
    assert!(matches!(
        repository.get_memo(&absolute),
        Err(Error::InvalidId(_))
    ));
}

#[test]
fn malformed_front_matter_reports_file_path_instead_of_skipping_it() {
    let (_directory, repository) = repository();
    let id = Path::new("memos").join("broken.md");
    let path = repository.root().join(&id);
    for invalid in [
        "body only",
        "---\ncreated_at: nope\n",
        "---\ncreated_at: nope\nupdated_at: nope\ntags: []\n---\n",
        "---\ncreated_at: 2026-01-01T00:00:00Z\n---\n",
    ] {
        fs::write(&path, invalid).unwrap();
        match repository.list_memos().unwrap_err() {
            Error::Format {
                path: error_path, ..
            } => assert_eq!(error_path, path),
            error => panic!("unexpected error: {error}"),
        }
    }
}

#[test]
fn malformed_settings_are_not_overwritten() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("kashikishi-tagbase-settings.toml");
    fs::write(&path, "invalid = [").unwrap();
    assert!(matches!(
        TagbaseMemo::new(directory.path()),
        Err(Error::Format { .. })
    ));
    assert_eq!(fs::read_to_string(path).unwrap(), "invalid = [");
}

#[test]
fn crlf_front_matter_and_body_separator_are_supported_without_trimming_body() {
    let (_directory, repository) = repository();
    let id = Path::new("memos").join("crlf.md");
    fs::write(repository.root().join(&id),
        "---\r\ncreated_at: 2026-01-01T00:00:00+09:00\r\nupdated_at: 2026-01-01T00:00:00+09:00\r\ntags: []\r\n---\r\n\r\n\r\nbody\r\n\r\n"
    ).unwrap();
    let memo = repository.get_memo(&id).unwrap();
    assert_eq!(memo.content.text, "\r\nbody\r\n\r\n");
    assert_eq!(
        memo.created_at,
        DateTime::parse_from_rfc3339("2026-01-01T00:00:00+09:00").unwrap()
    );
    let updated = repository
        .update_memo(&id, |content| content.title = Some("title".to_owned()))
        .unwrap();
    assert_eq!(
        repository.get_memo(&id).unwrap().content.text,
        updated.content.text
    );
}

#[test]
fn temporary_files_are_cleaned_up_and_non_markdown_files_are_ignored() {
    let (_directory, repository) = repository();
    let memo = repository.create_memo().unwrap();
    for index in 0..3 {
        repository
            .update_memo(&memo.id, |content| content.text = index.to_string())
            .unwrap();
    }
    let parent = repository.root().join(memo.id.parent().unwrap());
    assert_eq!(fs::read_dir(parent).unwrap().count(), 1);
    fs::write(
        repository.root().join("memos").join("notes.txt"),
        "not a memo",
    )
    .unwrap();
    assert_eq!(repository.list_memos().unwrap().len(), 1);
}

#[cfg(unix)]
#[test]
fn symlinks_are_rejected_without_writing_outside_the_repository() {
    use std::os::unix::fs::symlink;
    let (directory, repository) = repository();
    let outside = tempdir().unwrap();
    fs::remove_dir(directory.path().join("memos")).unwrap();
    symlink(outside.path(), directory.path().join("memos")).unwrap();
    assert!(matches!(repository.create_memo(), Err(Error::InvalidId(_))));
    assert!(matches!(repository.list_memos(), Err(Error::InvalidId(_))));
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
}
