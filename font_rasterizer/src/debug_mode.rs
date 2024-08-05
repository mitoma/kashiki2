use std::sync::LazyLock;

pub(crate) struct DebugFlags {
    pub(crate) show_glyph_outline: bool,
}

pub static DEBUG_FLAGS: LazyLock<DebugFlags> = LazyLock::new(|| {
    std::env::var("FONT_RASTERIZER_DEBUG")
        .map(|_debug| DebugFlags {
            show_glyph_outline: true,
        })
        .unwrap_or_else(|_| DebugFlags {
            show_glyph_outline: false,
        })
});
