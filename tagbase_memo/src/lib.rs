#![doc = include_str!("../README.md")]

mod query;

use std::{
    fs,
    io::{self, Write},
    path::{Component, Path, PathBuf},
};

use chrono::{DateTime, FixedOffset, Local};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use thiserror::Error;

use query::Query;

const SETTINGS_FILE: &str = "kashikishi-tagbase-settings.toml";

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{path}: {message}")]
    Format { path: PathBuf, message: String },
    #[error("Invalid memo ID or symbolic link: {0}")]
    InvalidId(PathBuf),
    #[error("Invalid search query: {0}")]
    InvalidQuery(String),
    #[error("Memo sequence exhausted: {0}")]
    SequenceExhausted(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub tag_definitions: Vec<String>,
    pub default_tag: String,
    pub invisible_tag: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            tag_definitions: ["scratch", "todo", "diary", "works", "archive"]
                .map(str::to_owned)
                .to_vec(),
            default_tag: "scratch".to_owned(),
            invisible_tag: "archive".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoContent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub tags: Vec<String>,
    #[serde(skip)]
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Memo {
    pub id: PathBuf,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
    pub content: MemoContent,
}

#[derive(Serialize, Deserialize)]
struct Metadata {
    created_at: DateTime<FixedOffset>,
    updated_at: DateTime<FixedOffset>,
    #[serde(flatten)]
    content: MemoContent,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

#[derive(Debug)]
pub struct TagbaseMemo {
    root: PathBuf,
    settings: Settings,
}

impl TagbaseMemo {
    /// 保管庫を開く。存在しないディレクトリと設定は初期値で作成する。
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        io_at(root, fs::create_dir_all(root))?;
        let root = io_at(root, fs::canonicalize(root))?;
        let memos = root.join("memos");
        io_at(&memos, fs::create_dir_all(&memos))?;
        reject_symlink(&memos)?;
        let settings_path = root.join(SETTINGS_FILE);
        let settings = match fs::read_to_string(&settings_path) {
            Ok(text) => toml::from_str(&text).map_err(|error| Error::Format {
                path: settings_path.clone(),
                message: error.to_string(),
            })?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let settings = Settings::default();
                let text = toml::to_string_pretty(&settings).map_err(|error| Error::Format {
                    path: settings_path.clone(),
                    message: error.to_string(),
                })?;
                let file = staged_file(&settings_path, &text)?;
                io_at(
                    &settings_path,
                    file.persist_noclobber(&settings_path).map_err(|e| e.error),
                )?;
                settings
            }
            Err(source) => {
                return Err(Error::Io {
                    path: settings_path,
                    source,
                });
            }
        };
        Ok(Self { root, settings })
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn create_memo(&self) -> Result<Memo> {
        let now = Local::now().fixed_offset();
        reject_symlink(&self.root.join("memos"))?;
        let directory = self
            .root
            .join("memos")
            .join(now.format("%Y-%m").to_string());
        io_at(&directory, fs::create_dir_all(&directory))?;
        reject_symlink(&directory)?;
        let prefix = format!("{}-", now.format("%Y-%m-%d"));
        let mut sequence = 0_u64;
        for entry in io_at(&directory, fs::read_dir(&directory))? {
            let entry = io_at(&directory, entry)?;
            let name = entry.file_name();
            if let Some(number) = name
                .to_str()
                .and_then(|name| name.strip_prefix(&prefix))
                .and_then(|name| name.strip_suffix(".md"))
                .and_then(|name| name.parse::<u64>().ok())
            {
                sequence = sequence.max(number);
            }
        }
        loop {
            sequence = sequence
                .checked_add(1)
                .ok_or_else(|| Error::SequenceExhausted(directory.clone()))?;
            let path = directory.join(format!("{prefix}{sequence:03}.md"));
            let memo = Memo {
                id: path
                    .strip_prefix(&self.root)
                    .expect("path belongs to repository")
                    .to_owned(),
                created_at: now,
                updated_at: now,
                content: MemoContent {
                    tags: vec![self.settings.default_tag.clone()],
                    ..MemoContent::default()
                },
            };
            let file = staged_file(&path, &encode(&memo, &path)?)?;
            match file.persist_noclobber(&path) {
                Ok(_) => return Ok(memo),
                Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(Error::Io {
                        path,
                        source: error.error,
                    });
                }
            }
        }
    }

    pub fn get_memo(&self, id: impl AsRef<Path>) -> Result<Memo> {
        let id = id.as_ref();
        let path = self.memo_path(id)?;
        decode(id, &path, &io_at(&path, fs::read_to_string(&path))?)
    }

    pub fn update_memo(
        &self,
        id: impl AsRef<Path>,
        update: impl FnOnce(&mut MemoContent),
    ) -> Result<Memo> {
        let mut memo = self.get_memo(id)?;
        let original = memo.content.clone();
        update(&mut memo.content);
        if original != memo.content {
            memo.updated_at = Local::now().fixed_offset();
            let path = self.memo_path(&memo.id)?;
            let file = staged_file(&path, &encode(&memo, &path)?)?;
            io_at(&path, file.persist(&path).map_err(|error| error.error))?;
        }
        Ok(memo)
    }

    pub fn delete_memo(&self, id: impl AsRef<Path>) -> Result<()> {
        let path = self.memo_path(id.as_ref())?;
        io_at(&path, fs::remove_file(&path))
    }

    pub fn list_memos(&self) -> Result<Vec<Memo>> {
        self.list_memos_with_order(SortOrder::Desc)
    }

    pub fn list_memos_with_order(&self, order: SortOrder) -> Result<Vec<Memo>> {
        self.select(Query::default(), order)
    }

    pub fn search(&self, input: &str) -> Result<Vec<Memo>> {
        self.search_with_order(input, SortOrder::Desc)
    }

    pub fn search_with_order(&self, input: &str, order: SortOrder) -> Result<Vec<Memo>> {
        let mut query = Query::parse(input)?;
        if query.keywords.is_empty() && query.tags.is_empty() {
            query.tags.push(self.settings.default_tag.clone());
        }
        self.select(query, order)
    }

    pub fn filter_by_tags(&self, tags: &[&str]) -> Result<Vec<Memo>> {
        self.filter_by_tags_with_order(tags, SortOrder::Desc)
    }

    pub fn filter_by_tags_with_order(&self, tags: &[&str], order: SortOrder) -> Result<Vec<Memo>> {
        let tags = if tags.is_empty() {
            vec![self.settings.default_tag.clone()]
        } else {
            tags.iter().map(|tag| (*tag).to_owned()).collect()
        };
        self.select(
            Query {
                tags,
                ..Query::default()
            },
            order,
        )
    }

    fn select(&self, query: Query, order: SortOrder) -> Result<Vec<Memo>> {
        let mut paths = Vec::new();
        collect_paths(&self.root.join("memos"), &mut paths)?;
        paths.sort();
        let show_hidden = query.tags.contains(&self.settings.invisible_tag);
        let mut memos = Vec::new();
        for path in paths {
            let id = path
                .strip_prefix(&self.root)
                .expect("path belongs to repository");
            let memo = self.get_memo(id)?;
            let content = &memo.content;
            if !show_hidden && content.tags.contains(&self.settings.invisible_tag) {
                continue;
            }
            if !query.tags.iter().all(|tag| content.tags.contains(tag)) {
                continue;
            }
            let title = content.title.as_deref().unwrap_or("").to_lowercase();
            let text = content.text.to_lowercase();
            if query
                .keywords
                .iter()
                .all(|word| title.contains(word) || text.contains(word))
            {
                memos.push(memo);
            }
        }
        memos.sort_by(|left, right| {
            let date = match order {
                SortOrder::Asc => left.created_at.cmp(&right.created_at),
                SortOrder::Desc => right.created_at.cmp(&left.created_at),
            };
            date.then_with(|| left.id.file_name().cmp(&right.id.file_name()))
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(memos)
    }

    fn memo_path(&self, id: &Path) -> Result<PathBuf> {
        let mut parts = id.components();
        if parts.next() != Some(Component::Normal("memos".as_ref()))
            || !parts
                .clone()
                .all(|part| matches!(part, Component::Normal(_)))
            || id.extension().is_none_or(|extension| extension != "md")
        {
            return Err(Error::InvalidId(id.to_owned()));
        }
        let mut path = self.root.join("memos");
        reject_symlink(&path)?;
        for part in parts {
            path.push(part);
            reject_symlink(&path)?;
        }
        Ok(path)
    }
}

fn io_at<T>(path: &Path, result: io::Result<T>) -> Result<T> {
    result.map_err(|source| Error::Io {
        path: path.to_owned(),
        source,
    })
}

fn reject_symlink(path: &Path) -> Result<()> {
    if io_at(path, fs::symlink_metadata(path))?
        .file_type()
        .is_symlink()
    {
        return Err(Error::InvalidId(path.to_owned()));
    }
    Ok(())
}

fn collect_paths(directory: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    reject_symlink(directory)?;
    for entry in io_at(directory, fs::read_dir(directory))? {
        let entry = io_at(directory, entry)?;
        let path = entry.path();
        let kind = io_at(&path, entry.file_type())?;
        if kind.is_symlink() {
            return Err(Error::InvalidId(path));
        }
        if kind.is_dir() {
            collect_paths(&path, paths)?;
        } else if path.extension().is_some_and(|extension| extension == "md") {
            paths.push(path);
        }
    }
    Ok(())
}

fn staged_file(path: &Path, text: &str) -> Result<NamedTempFile> {
    let parent = path.parent().expect("repository file has a parent");
    let mut file = io_at(path, NamedTempFile::new_in(parent))?;
    io_at(path, file.write_all(text.as_bytes()))?;
    io_at(path, file.as_file().sync_all())?;
    Ok(file)
}

fn encode(memo: &Memo, path: &Path) -> Result<String> {
    let metadata = Metadata {
        created_at: memo.created_at,
        updated_at: memo.updated_at,
        content: memo.content.clone(),
    };
    let yaml = serde_yaml_ng::to_string(&metadata).map_err(|error| Error::Format {
        path: path.to_owned(),
        message: error.to_string(),
    })?;
    Ok(format!("---\n{yaml}---\n\n{}", memo.content.text))
}

fn decode(id: &Path, path: &Path, text: &str) -> Result<Memo> {
    let invalid = |message: &str| Error::Format {
        path: path.to_owned(),
        message: message.to_owned(),
    };
    let first = text
        .find('\n')
        .ok_or_else(|| invalid("Missing YAML front matter"))?;
    if text[..first].trim_end_matches('\r') != "---" {
        return Err(invalid("Missing YAML front matter"));
    }
    let mut offset = first + 1;
    for line in text[offset..].split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            let metadata: Metadata =
                serde_yaml_ng::from_str(&text[first + 1..offset]).map_err(|error| {
                    Error::Format {
                        path: path.to_owned(),
                        message: error.to_string(),
                    }
                })?;
            let body = &text[offset + line.len()..];
            let body = body
                .strip_prefix("\r\n")
                .or_else(|| body.strip_prefix('\n'))
                .unwrap_or(body);
            return Ok(Memo {
                id: id.to_owned(),
                created_at: metadata.created_at,
                updated_at: metadata.updated_at,
                content: MemoContent {
                    text: body.to_owned(),
                    ..metadata.content
                },
            });
        }
        offset += line.len();
    }
    Err(invalid("Unclosed YAML front matter"))
}
