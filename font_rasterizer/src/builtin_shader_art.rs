/// 組み込みシェーダーアートの定義
pub struct BuiltinShaderArt {
    /// 識別名（設定ファイルや `change-shader-art` コマンドで使用）
    pub name: &'static str,
    /// UI に表示する名前
    pub display_name: &'static str,
    /// WGSL シェーダーソース
    pub source: &'static str,
}

/// 組み込みシェーダーアートの一覧
pub static BUILTIN_SHADERS: &[BuiltinShaderArt] = &[
    BuiltinShaderArt {
        name: "gradient",
        display_name: "グラデーション (テンプレート)",
        source: include_str!("shader/sa_rainbow.wgsl"),
    },
    BuiltinShaderArt {
        name: "starfield_warp",
        display_name: "スターフィールドワープ",
        source: include_str!("shader/sa_starfield_warp.wgsl"),
    },
    BuiltinShaderArt {
        name: "pale_snowfall",
        display_name: "淡い雪景色",
        source: include_str!("shader/sa_pale_snowfall.wgsl"),
    },
];

/// 識別名からシェーダーソースを取得する。見つからない場合は `None`。
pub fn find_by_name(name: &str) -> Option<&'static str> {
    BUILTIN_SHADERS
        .iter()
        .find(|s| s.name == name)
        .map(|s| s.source)
}
