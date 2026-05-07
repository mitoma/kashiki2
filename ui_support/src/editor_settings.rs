use font_rasterizer::{color_theme::ColorTheme, glyph_vertex_buffer::Direction};
use serde::{Deserialize, Serialize};

use crate::ui_context::{
    CharEasings, CharEasingsPreset, GpuEasingConfig, HighlightMode, TextContext,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTextContextProfile {
    Document,
    ModalLabel,
    ModalInput,
    SelectBoxSearch,
    SelectBoxItem,
    ImePreedit,
    Card,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorDirection {
    #[default]
    Horizontal,
    Vertical,
}

impl From<EditorDirection> for Direction {
    fn from(value: EditorDirection) -> Self {
        match value {
            EditorDirection::Horizontal => Direction::Horizontal,
            EditorDirection::Vertical => Direction::Vertical,
        }
    }
}

impl From<Direction> for EditorDirection {
    fn from(value: Direction) -> Self {
        match value {
            Direction::Horizontal => EditorDirection::Horizontal,
            Direction::Vertical => EditorDirection::Vertical,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorColorTheme {
    SolarizedLight,
    #[default]
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

impl From<EditorColorTheme> for ColorTheme {
    fn from(value: EditorColorTheme) -> Self {
        match value {
            EditorColorTheme::SolarizedLight => ColorTheme::SolarizedLight,
            EditorColorTheme::SolarizedDark => ColorTheme::SolarizedDark,
            EditorColorTheme::SolarizedBlackback => ColorTheme::SolarizedBlackback,
            EditorColorTheme::HighContrastLight => ColorTheme::HighContrastLight,
            EditorColorTheme::HighContrastDark => ColorTheme::HighContrastDark,
            EditorColorTheme::WarmLight => ColorTheme::WarmLight,
            EditorColorTheme::WarmDark => ColorTheme::WarmDark,
            EditorColorTheme::CoolLight => ColorTheme::CoolLight,
            EditorColorTheme::CoolDark => ColorTheme::CoolDark,
            EditorColorTheme::Vivid => ColorTheme::Vivid,
        }
    }
}

impl From<ColorTheme> for EditorColorTheme {
    fn from(value: ColorTheme) -> Self {
        match value {
            ColorTheme::SolarizedLight => EditorColorTheme::SolarizedLight,
            ColorTheme::SolarizedDark => EditorColorTheme::SolarizedDark,
            ColorTheme::SolarizedBlackback => EditorColorTheme::SolarizedBlackback,
            ColorTheme::HighContrastLight => EditorColorTheme::HighContrastLight,
            ColorTheme::HighContrastDark => EditorColorTheme::HighContrastDark,
            ColorTheme::WarmLight => EditorColorTheme::WarmLight,
            ColorTheme::WarmDark => EditorColorTheme::WarmDark,
            ColorTheme::CoolLight => EditorColorTheme::CoolLight,
            ColorTheme::CoolDark => EditorColorTheme::CoolDark,
            ColorTheme::Vivid => EditorColorTheme::Vivid,
            ColorTheme::Custom { .. } => EditorColorTheme::SolarizedDark,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorHighlightMode {
    #[default]
    None,
    Markdown,
    Language(String),
}

impl From<EditorHighlightMode> for HighlightMode {
    fn from(value: EditorHighlightMode) -> Self {
        match value {
            EditorHighlightMode::None => HighlightMode::None,
            EditorHighlightMode::Markdown => HighlightMode::Markdown,
            EditorHighlightMode::Language(language) => HighlightMode::Language(language),
        }
    }
}

impl From<HighlightMode> for EditorHighlightMode {
    fn from(value: HighlightMode) -> Self {
        match value {
            HighlightMode::None => EditorHighlightMode::None,
            HighlightMode::Markdown => EditorHighlightMode::Markdown,
            HighlightMode::Language(language) => EditorHighlightMode::Language(language),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorTextContextSettings {
    pub row_interval: f32,
    pub col_interval: f32,
    pub row_scale: f32,
    pub col_scale: f32,
    pub max_col: usize,
    pub min_bound: [f32; 2],
    pub char_easings_preset: CharEasingsPreset,
    pub psychedelic: bool,
    pub hyde_caret: bool,
    pub highlight_mode: EditorHighlightMode,
}

impl Default for EditorTextContextSettings {
    fn default() -> Self {
        let default = TextContext::default();
        Self {
            row_interval: default.row_interval,
            col_interval: default.col_interval,
            row_scale: default.row_scale,
            col_scale: default.col_scale,
            max_col: default.max_col,
            min_bound: default.min_bound.to_array(),
            char_easings_preset: CharEasingsPreset::Default,
            psychedelic: default.psychedelic,
            hyde_caret: default.hyde_caret,
            highlight_mode: default.highlight_mode.into(),
        }
    }
}

impl EditorTextContextSettings {
    fn apply_to(&self, context: &mut TextContext) {
        context.row_interval = self.row_interval;
        context.col_interval = self.col_interval;
        context.row_scale = self.row_scale;
        context.col_scale = self.col_scale;
        context.max_col = self.max_col;
        context.min_bound = self.min_bound.into();
        context.char_easings = CharEasings::from_preset(self.char_easings_preset);
        context.psychedelic = self.psychedelic;
        context.hyde_caret = self.hyde_caret;
        context.highlight_mode = self.highlight_mode.clone().into();
    }

    fn to_text_context(&self, direction: Direction, color_theme: ColorTheme) -> TextContext {
        let mut context = TextContext {
            direction,
            color_theme,
            ..Default::default()
        };
        self.apply_to(&mut context);
        context
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorTextContextPatch {
    pub direction: Option<EditorDirection>,
    pub row_interval: Option<f32>,
    pub col_interval: Option<f32>,
    pub row_scale: Option<f32>,
    pub col_scale: Option<f32>,
    pub max_col: Option<usize>,
    pub min_bound: Option<[f32; 2]>,
    pub char_easings_preset: Option<CharEasingsPreset>,
    pub psychedelic: Option<bool>,
    pub hyde_caret: Option<bool>,
    pub highlight_mode: Option<EditorHighlightMode>,
}

impl EditorTextContextPatch {
    fn apply_to(&self, context: &mut TextContext) {
        if let Some(direction) = self.direction {
            context.direction = direction.into();
        }
        if let Some(row_interval) = self.row_interval {
            context.row_interval = row_interval;
        }
        if let Some(col_interval) = self.col_interval {
            context.col_interval = col_interval;
        }
        if let Some(row_scale) = self.row_scale {
            context.row_scale = row_scale;
        }
        if let Some(col_scale) = self.col_scale {
            context.col_scale = col_scale;
        }
        if let Some(max_col) = self.max_col {
            context.max_col = max_col;
        }
        if let Some(min_bound) = self.min_bound {
            context.min_bound = min_bound.into();
        }
        if let Some(preset) = self.char_easings_preset {
            context.char_easings = CharEasings::from_preset(preset);
        }
        if let Some(psychedelic) = self.psychedelic {
            context.psychedelic = psychedelic;
        }
        if let Some(hyde_caret) = self.hyde_caret {
            context.hyde_caret = hyde_caret;
        }
        if let Some(highlight_mode) = self.highlight_mode.clone() {
            context.highlight_mode = highlight_mode.into();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorSettingsProfiles {
    pub document: EditorTextContextPatch,
    pub modal_label: EditorTextContextPatch,
    pub modal_input: EditorTextContextPatch,
    pub selectbox_search: EditorTextContextPatch,
    pub selectbox_item: EditorTextContextPatch,
    pub ime_preedit: EditorTextContextPatch,
    pub card: EditorTextContextPatch,
}

impl Default for EditorSettingsProfiles {
    fn default() -> Self {
        Self {
            document: EditorTextContextPatch::default(),
            modal_label: EditorTextContextPatch {
                max_col: Some(usize::MAX),
                min_bound: Some([1.0, 1.0]),
                char_easings_preset: Some(CharEasingsPreset::ZeroMotion),
                hyde_caret: Some(true),
                highlight_mode: Some(EditorHighlightMode::None),
                ..Default::default()
            },
            modal_input: EditorTextContextPatch {
                min_bound: Some([1.0, 1.0]),
                ..Default::default()
            },
            selectbox_search: EditorTextContextPatch {
                min_bound: Some([1.0, 1.0]),
                ..Default::default()
            },
            selectbox_item: EditorTextContextPatch {
                max_col: Some(usize::MAX),
                min_bound: Some([1.0, 1.0]),
                hyde_caret: Some(true),
                highlight_mode: Some(EditorHighlightMode::None),
                ..Default::default()
            },
            ime_preedit: EditorTextContextPatch {
                max_col: Some(usize::MAX),
                min_bound: Some([1.0, 10.0]),
                char_easings_preset: Some(CharEasingsPreset::IgnoreCamera),
                hyde_caret: Some(true),
                highlight_mode: Some(EditorHighlightMode::None),
                ..Default::default()
            },
            card: EditorTextContextPatch {
                max_col: Some(usize::MAX),
                min_bound: Some([1.0, 10.0]),
                char_easings_preset: Some(CharEasingsPreset::IgnoreCamera),
                hyde_caret: Some(true),
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorSettings {
    pub color_theme: EditorColorTheme,
    pub global_direction: EditorDirection,
    pub base_text_context: EditorTextContextSettings,
    pub profiles: EditorSettingsProfiles,
}

impl EditorSettings {
    pub fn color_theme(&self) -> ColorTheme {
        self.color_theme.into()
    }

    pub fn global_direction(&self) -> Direction {
        self.global_direction.into()
    }

    pub fn set_color_theme(&mut self, color_theme: ColorTheme) {
        self.color_theme = color_theme.into();
    }

    pub fn set_global_direction(&mut self, direction: Direction) {
        self.global_direction = direction.into();
    }

    pub fn text_context(&self, profile: EditorTextContextProfile) -> TextContext {
        let mut context = self
            .base_text_context
            .to_text_context(self.global_direction(), self.color_theme());
        let patch = match profile {
            EditorTextContextProfile::Document => &self.profiles.document,
            EditorTextContextProfile::ModalLabel => &self.profiles.modal_label,
            EditorTextContextProfile::ModalInput => &self.profiles.modal_input,
            EditorTextContextProfile::SelectBoxSearch => &self.profiles.selectbox_search,
            EditorTextContextProfile::SelectBoxItem => &self.profiles.selectbox_item,
            EditorTextContextProfile::ImePreedit => &self.profiles.ime_preedit,
            EditorTextContextProfile::Card => &self.profiles.card,
        };
        patch.apply_to(&mut context);

        if matches!(profile, EditorTextContextProfile::SelectBoxItem) {
            context.char_easings = CharEasings {
                select_char: GpuEasingConfig::default(),
                unselect_char: GpuEasingConfig::default(),
                add_char: GpuEasingConfig::fadein(),
                remove_char: GpuEasingConfig::fadeout(),
                ..CharEasings::default()
            };
        }

        context
    }
}
