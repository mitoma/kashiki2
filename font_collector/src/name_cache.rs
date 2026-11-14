use std::collections::{HashMap, HashSet};
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use log::{info, warn};
use serde::{Deserialize, Serialize};

use crate::convert_text::PreferredLanguage;

// キャッシュファイルの形式やフォント名の抽出ロジックを変えた場合はインクリメントする
const CACHE_VERSION: u32 = 1;

/// フォントファイルが変更されたかどうかを判定するためのスタンプ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FileStamp {
    len: u64,
    modified_secs: u64,
    modified_nanos: u32,
}

impl FileStamp {
    pub(crate) fn from_metadata(metadata: &Metadata) -> Option<Self> {
        let modified = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
        Some(Self {
            len: metadata.len(),
            modified_secs: modified.as_secs(),
            modified_nanos: modified.subsec_nanos(),
        })
    }
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    path: PathBuf,
    stamp: FileStamp,
    names: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct CacheFile {
    version: u32,
    language: String,
    entries: Vec<CacheEntry>,
}

fn language_key(language: Option<PreferredLanguage>) -> String {
    match language {
        Some(language) => format!("{language:?}"),
        None => String::from("None"),
    }
}

/// フォントファイルのパスからフォント名一覧を引くキャッシュ。
/// ファイルパスが設定されていればディスクにも永続化する。
#[derive(Default)]
pub(crate) struct FontNameCache {
    file_path: Option<PathBuf>,
    loaded: bool,
    dirty: bool,
    entries: HashMap<PathBuf, (FileStamp, Vec<String>)>,
}

impl FontNameCache {
    pub(crate) fn set_file_path(&mut self, path: PathBuf) {
        self.file_path = Some(path);
        self.loaded = false;
    }

    pub(crate) fn ensure_loaded(&mut self, language: Option<PreferredLanguage>) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        let Some(file_path) = &self.file_path else {
            return;
        };
        let data = match std::fs::read(file_path) {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
            Err(e) => {
                warn!("フォント名キャッシュの読み込みに失敗: {e}");
                return;
            }
        };
        let cache_file: CacheFile = match serde_json::from_slice(&data) {
            Ok(cache_file) => cache_file,
            Err(e) => {
                warn!("フォント名キャッシュの解析に失敗: {e}");
                return;
            }
        };
        if cache_file.version != CACHE_VERSION || cache_file.language != language_key(language) {
            info!("フォント名キャッシュの形式が異なるため破棄します");
            return;
        }
        for entry in cache_file.entries {
            self.entries
                .entry(entry.path)
                .or_insert((entry.stamp, entry.names));
        }
        info!(
            "フォント名キャッシュを読み込みました: {} entries",
            self.entries.len()
        );
    }

    pub(crate) fn get(&self, path: &Path, stamp: FileStamp) -> Option<&Vec<String>> {
        self.entries
            .get(path)
            .filter(|(cached_stamp, _)| *cached_stamp == stamp)
            .map(|(_, names)| names)
    }

    pub(crate) fn insert(&mut self, path: PathBuf, stamp: FileStamp, names: Vec<String>) {
        self.entries.insert(path, (stamp, names));
        self.dirty = true;
    }

    /// 走査で見つからなかった（削除された）フォントファイルのエントリを取り除く
    pub(crate) fn retain(&mut self, paths: &HashSet<PathBuf>) {
        let before = self.entries.len();
        self.entries.retain(|path, _| paths.contains(path));
        if self.entries.len() != before {
            self.dirty = true;
        }
    }

    pub(crate) fn save_if_dirty(&mut self, language: Option<PreferredLanguage>) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        let Some(file_path) = &self.file_path else {
            return;
        };
        let mut entries = self
            .entries
            .iter()
            .map(|(path, (stamp, names))| CacheEntry {
                path: path.clone(),
                stamp: *stamp,
                names: names.clone(),
            })
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        let cache_file = CacheFile {
            version: CACHE_VERSION,
            language: language_key(language),
            entries,
        };
        if let Err(e) = write_cache_file(file_path, &cache_file) {
            warn!("フォント名キャッシュの書き込みに失敗: {e}");
        }
    }
}

fn write_cache_file(file_path: &Path, cache_file: &CacheFile) -> std::io::Result<()> {
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_vec(cache_file).map_err(std::io::Error::other)?;
    // 書き込み途中のファイルを読まないよう一時ファイル経由で置き換える
    let tmp_path = file_path.with_extension("tmp");
    std::fs::write(&tmp_path, data)?;
    std::fs::rename(&tmp_path, file_path)
}
