use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::memos::Memos;

#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Memo {
    #[serde(with = "crate::local_datetime_format")]
    pub(crate) created_at: DateTime<Local>,
    pub(crate) title: Option<String>,
    pub(crate) text: String,
}

impl Default for Memo {
    fn default() -> Self {
        Memo {
            created_at: Local::now(),
            title: None,
            text: "".to_string(),
        }
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CategorizedMemos {
    pub(crate) current_category: String,
    pub(crate) categorized: BTreeMap<String, Memos>,
}

const DEFAULT_CATEGORY: &str = "default";

impl CategorizedMemos {
    fn new() -> Self {
        CategorizedMemos {
            current_category: DEFAULT_CATEGORY.to_string(),
            categorized: BTreeMap::from([(DEFAULT_CATEGORY.to_string(), Memos::default())]),
        }
    }

    pub(crate) fn load_memos() -> CategorizedMemos {
        let memos_file = memos_file();
        if memos_file.exists() {
            // Read memos from file
            let memos_json = fs::read_to_string(memos_file).unwrap();
            serde_json::from_str(&memos_json).unwrap()
        } else {
            // ファイルが存在しない時は、親ディレクトリまで作成してからファイルを作る
            let memos_dir = memos_file.parent().unwrap();
            fs::create_dir_all(memos_dir).unwrap();

            let memos = CategorizedMemos::new();
            let memos_json = serde_json::to_string(&memos).unwrap();
            fs::write(memos_file, memos_json).unwrap();
            memos
        }
    }

    pub(crate) fn save_memos(&self) -> Result<(), std::io::Error> {
        if Self::load_memos() == *self {
            return Ok(());
        }

        let memos_file = memos_file();
        // 上記のファイルを memos.[現在日時].json にリネームして保存する
        let now = chrono::Local::now();
        let memos_file_backup =
            memos_file.with_extension(format!("{}.json", now.format("%Y%m%d%H%M%S")));
        fs::rename(&memos_file, memos_file_backup)?;

        let memos_json = serde_json::to_string(self).unwrap();
        fs::write(memos_file, memos_json)
    }

    pub(crate) fn update_memos(&mut self, category: &str, memos: Memos) {
        self.categorized.insert(category.to_string(), memos);
    }

    pub(crate) fn get_current_memos(&self) -> Option<&Memos> {
        self.categorized.get(self.current_category.as_str())
    }

    pub(crate) fn add_memo(&mut self, category: Option<&str>, content: String) {
        self.categorized
            .entry(category.unwrap_or("default").to_string())
            .or_insert(Memos::empty())
            .memos
            .push(content);
    }

    pub(crate) fn categories(&self) -> Vec<String> {
        self.categorized.keys().cloned().collect()
    }
}

fn memos_file() -> PathBuf {
    // いわゆるホームディレクトリのパスを取得する
    let home_dir = dirs::home_dir().unwrap();
    Path::new(&home_dir).join(".config/kashikishi/categorized_memos.json")
}
