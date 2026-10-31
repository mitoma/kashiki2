use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 設定やメモを保存するディレクトリを差し替える。最初にパスを解決する前に呼ぶ必要がある。
pub(crate) fn set_data_dir(dir: PathBuf) -> Result<(), PathBuf> {
    DATA_DIR.set(dir)
}

/// 設定やメモを保存するディレクトリ。既定値は `~/.config/kashikishi`
pub(crate) fn data_dir() -> &'static Path {
    DATA_DIR.get_or_init(|| {
        // いわゆるホームディレクトリのパスを取得する
        let home_dir = dirs::home_dir().unwrap();
        home_dir.join(".config/kashikishi")
    })
}

pub(crate) fn config_file() -> PathBuf {
    data_dir().join("config.json")
}

pub(crate) fn categorized_memos_file() -> PathBuf {
    data_dir().join("categorized_memos.json")
}
