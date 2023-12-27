use std::sync::Mutex;

use once_cell::sync::Lazy;

pub(crate) struct DebugFlags {
    pub(crate) show_glyph_outline: bool,
}

pub static DEBUG_FLAGS: Lazy<Mutex<DebugFlags>> = Lazy::new(defualt_debug_flags);

fn defualt_debug_flags() -> Mutex<DebugFlags> {
    let flags = std::env::var("FONT_RASTERIZER_DEBUG")
        .map(|_debug| DebugFlags {
            show_glyph_outline: true,
        })
        .unwrap_or_else(|_| DebugFlags {
            show_glyph_outline: false,
        });
    Mutex::new(flags)
}
