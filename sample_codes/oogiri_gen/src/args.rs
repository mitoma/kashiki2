use clap::{Parser, ValueEnum};
use font_rasterizer::{color_theme::ColorTheme, context::WindowSize};
use ui_support::ui_context::CharEasingsPreset;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "generator", long_about = None)]
pub struct Args {
    /// font
    #[arg(
        short,
        long,
        default_value = "あしびきの\n山鳥の尾の\nしだり尾の\nながながし夜を\nひとりかも寝む"
    )]
    pub target_string: String,
    #[arg(short, long, default_value = "default")]
    pub preset: CharEasingsPresetArg,
    #[arg(short, long, default_value = "solarized-dark")]
    pub color_theme: ColorThemeArg,
    #[arg(short, long, default_value = "square")]
    pub window_size: WindowSizeArg,
}

#[derive(Clone, Debug, ValueEnum, Default)]
pub enum CharEasingsPresetArg {
    /// デフォルトのアニメーション設定
    #[default]
    Default,
    Poppy,
    Cool,
    Energetic,
    Gentle,
    Minimal,
}

impl From<CharEasingsPresetArg> for CharEasingsPreset {
    fn from(arg: CharEasingsPresetArg) -> Self {
        match arg {
            CharEasingsPresetArg::Default => CharEasingsPreset::Default,
            CharEasingsPresetArg::Poppy => CharEasingsPreset::Poppy,
            CharEasingsPresetArg::Cool => CharEasingsPreset::Cool,
            CharEasingsPresetArg::Energetic => CharEasingsPreset::Energetic,
            CharEasingsPresetArg::Gentle => CharEasingsPreset::Gentle,
            CharEasingsPresetArg::Minimal => CharEasingsPreset::Minimal,
        }
    }
}

#[derive(Clone, Debug, ValueEnum, Default)]
pub enum ColorThemeArg {
    #[default]
    SolarizedLight,
    SolarizedDark,
    SolarizedBlackback,
    HighContrastLight,
    HighContrastDark,
    WarmLight,
    WarmDark,
    CoolLight,
    CoolDark,
    Vivid,
}

impl From<ColorThemeArg> for ColorTheme {
    fn from(value: ColorThemeArg) -> Self {
        match value {
            ColorThemeArg::SolarizedLight => ColorTheme::SolarizedLight,
            ColorThemeArg::SolarizedDark => ColorTheme::SolarizedDark,
            ColorThemeArg::SolarizedBlackback => ColorTheme::SolarizedBlackback,
            ColorThemeArg::HighContrastLight => ColorTheme::HighContrastLight,
            ColorThemeArg::HighContrastDark => ColorTheme::HighContrastDark,
            ColorThemeArg::WarmLight => ColorTheme::WarmLight,
            ColorThemeArg::WarmDark => ColorTheme::WarmDark,
            ColorThemeArg::CoolLight => ColorTheme::CoolLight,
            ColorThemeArg::CoolDark => ColorTheme::CoolDark,
            ColorThemeArg::Vivid => ColorTheme::Vivid,
        }
    }
}

#[derive(Clone, Debug, ValueEnum, Default)]
pub enum WindowSizeArg {
    #[default]
    Square,
    Square4x3,
    Square3x4,
    Wide16x9,
    Wide9x16,
}

impl From<WindowSizeArg> for WindowSize {
    fn from(arg: WindowSizeArg) -> Self {
        match arg {
            WindowSizeArg::Square => WindowSize::new(600, 600),
            WindowSizeArg::Square4x3 => WindowSize::new(800, 600),
            WindowSizeArg::Wide16x9 => WindowSize::new(800, 450),
            WindowSizeArg::Square3x4 => WindowSize::new(600, 800),
            WindowSizeArg::Wide9x16 => WindowSize::new(450, 800),
        }
    }
}
