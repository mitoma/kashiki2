use font_rasterizer::{
    context::WindowSize, glyph_instances::GlyphInstances, time::now_millis,
    vector_instances::VectorInstances,
};
use ui_support::{
    editor_settings::EditorColorTheme,
    layout_engine::{
        ModelBorder,
        ModelSurfaceStyle::{self},
    },
    ui::{Card, CardAnchor},
    ui_context::UiContext,
};

// FPS が毎フレーム書き換わって読みにくくならないよう、この間隔でだけ内容を更新する
const REFRESH_INTERVAL_MILLIS: u32 = 500;

/// 画面左上に炊紙自体のシステム情報を表示するカード
pub(crate) struct SystemInfoCard {
    card: Card,
    text: Option<String>,
    last_refresh: Option<u32>,
}

impl SystemInfoCard {
    pub(crate) fn new(context: &UiContext) -> Self {
        let mut card = Card::with_settings(context.editor_settings());

        card.set_border(ModelBorder::Rounded);
        card.set_anchor(CardAnchor::TopLeft);
        card.set_surface_style(ModelSurfaceStyle::OPAQUE_THEME_BACKGROUND_HIGHLIGHT);
        Self {
            card,
            text: None,
            last_refresh: None,
        }
    }

    pub(crate) fn update(&mut self, context: &UiContext) {
        let now = now_millis();
        let refresh = self
            .last_refresh
            .is_none_or(|last| now.wrapping_sub(last) >= REFRESH_INTERVAL_MILLIS);
        if refresh {
            self.last_refresh = Some(now);
            let font_repository = context.font_repository();
            let text = system_info_text(
                context.window_size(),
                font_rasterizer::time::fps(),
                context.editor_settings().color_theme,
                &font_repository.font_names(),
                font_repository.ascii_override_font_name(),
            );
            if self.text.as_ref() != Some(&text) {
                context.register_string(text.clone());
                self.card.set_text(context, text.clone());
                self.text = Some(text);
            }
        }
        self.card.update(context);
    }

    pub(crate) fn get_instances(&self) -> Vec<&GlyphInstances> {
        self.card.get_instances()
    }

    pub(crate) fn get_vector_instances(&self) -> Vec<&VectorInstances<String>> {
        self.card.get_vector_instances()
    }

    pub(crate) fn get_surface_instances(&self) -> Vec<&VectorInstances<String>> {
        self.card.get_surface_instances()
    }
}

fn system_info_text(
    window_size: WindowSize,
    fps: f32,
    color_theme: EditorColorTheme,
    font_names: &[&str],
    ascii_override_font_name: Option<&str>,
) -> String {
    let mut lines = vec![
        format!("解像度: {}x{}", window_size.width, window_size.height),
        format!("FPS: {fps:.0}"),
        format!("テーマ: {color_theme:?}"),
    ];
    // 先頭のフォントが優先され、残りはグリフが無いときのフォールバックとして使われる
    match font_names.split_first() {
        Some((primary, fallbacks)) => {
            lines.push(format!("フォント: {primary}"));
            if !fallbacks.is_empty() {
                lines.push(format!("フォールバック: {}", fallbacks.join(", ")));
            }
        }
        None => lines.push("フォント: なし".to_string()),
    }
    if let Some(name) = ascii_override_font_name {
        lines.push(format!("ASCII フォント: {name}"));
    }
    lines.join("\n")
}
