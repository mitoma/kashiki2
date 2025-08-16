use std::sync::LazyLock;

pub(crate) struct DebugFlags {
    pub(crate) show_glyph_outline: bool,
    pub(crate) debug_shader: bool,
}

pub static DEBUG_FLAGS: LazyLock<DebugFlags> = LazyLock::new(|| DebugFlags {
    show_glyph_outline: std::env::var("FONT_RASTERIZER_DEBUG")
        .map(|_debug| true)
        .unwrap_or(false),
    debug_shader: std::env::var("FONT_RASTERIZER_DEBUG_SHADER")
        .map(|_debug| true)
        .unwrap_or(false),
});
