use once_cell::sync::Lazy;

pub(crate) struct DebugFlags {
    pub(crate) show_glyph_outline: bool,
}

pub static DEBUG_FLAGS: Lazy<DebugFlags> = Lazy::new(defualt_debug_flags);

fn defualt_debug_flags() -> DebugFlags {
    std::env::var("FONT_RASTERIZER_DEBUG")
        .map(|_debug| DebugFlags {
            show_glyph_outline: true,
        })
        .unwrap_or_else(|_| DebugFlags {
            show_glyph_outline: false,
        })
}
