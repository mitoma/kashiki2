use serde::{Deserialize, Serialize};
use ui_support::layout_engine::World;

#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Memos {
    pub(crate) memos: Vec<String>,
}

impl Default for Memos {
    fn default() -> Self {
        Memos {
            memos: vec!["".to_string()],
        }
    }
}

impl Memos {
    pub(crate) fn empty() -> Self {
        Memos { memos: vec![] }
    }
}

impl From<&dyn World> for Memos {
    fn from(world: &dyn World) -> Self {
        Memos {
            memos: world.strings(),
        }
    }
}

/*
fn memos_file() -> PathBuf {
    // いわゆるホームディレクトリのパスを取得する
    let home_dir = dirs::home_dir().unwrap();
    Path::new(&home_dir).join(".config/kashikishi/memos.json")
    //Path::new(&home_dir).join(".config/kashikishi/debug.json")
}

// $HOME/.config/kashikishi/memos.json に保存されたメモを読み込む
pub(crate) fn load_memos() -> Memos {
    let memos_file = memos_file();
    let memos: Vec<String>;

    if memos_file.exists() {
        // Read memos from file
        let memos_json = fs::read_to_string(memos_file).unwrap();
        memos = serde_json::from_str(&memos_json).unwrap();
    } else {
        // ファイルが存在しない時は、親ディレクトリまで作成してからファイルを作る
        let memos_dir = memos_file.parent().unwrap();
        fs::create_dir_all(memos_dir).unwrap();

        // Set memos to [""] and save to file
        memos = vec!["".to_string()];
        let memos_json = serde_json::to_string(&memos).unwrap();
        fs::write(memos_file, memos_json).unwrap();
    }
    Memos { memos }
}

pub(crate) fn save_memos(memos: Memos) -> Result<(), std::io::Error> {
    if load_memos().memos == memos.memos {
        return Ok(());
    }

    let memos_file = memos_file();
    // 上記のファイルを memos.[現在日時].json にリネームして保存する
    let now = chrono::Local::now();
    let memos_file_backup =
        memos_file.with_extension(format!("{}.json", now.format("%Y%m%d%H%M%S")));
    fs::rename(&memos_file, memos_file_backup)?;

    let memos_json = serde_json::to_string(&memos.memos).unwrap();
    fs::write(memos_file, memos_json)
}
 */
