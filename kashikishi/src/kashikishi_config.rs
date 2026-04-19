use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct KashikishiConfig {
    pub(crate) ime_on_the_fly: bool,
    pub(crate) font: Option<String>,
    pub(crate) ascii_override_font: Option<String>,
}

impl Default for KashikishiConfig {
    fn default() -> Self {
        KashikishiConfig {
            ime_on_the_fly: true,
            font: None,
            ascii_override_font: None,
        }
    }
}

impl KashikishiConfig {
    pub fn load() -> Self {
        let config_file = Self::config_file();
        if config_file.exists() {
            let config_json = std::fs::read_to_string(config_file).unwrap();
            serde_json::from_str(&config_json).unwrap()
        } else {
            let config = KashikishiConfig::default();
            let config_json = serde_json::to_string(&config).unwrap();
            std::fs::write(Self::config_file(), config_json).unwrap();
            config
        }
    }

    pub fn save(&self) {
        let config_json = serde_json::to_string(self).unwrap();
        std::fs::write(Self::config_file(), config_json).unwrap();
    }

    fn config_file() -> std::path::PathBuf {
        // いわゆるホームディレクトリのパスを取得する
        let home_dir = dirs::home_dir().unwrap();
        std::path::Path::new(&home_dir).join(".config/kashikishi/config.json")
    }
}
