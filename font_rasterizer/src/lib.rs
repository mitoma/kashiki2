mod background_bind_group;
pub mod builtin_shader_art;
pub mod char_width_calcurator;
pub mod color_theme;
pub mod context;
mod debug_mode;
pub mod errors;
pub mod font_converter;
#[cfg(all(feature = "cache", not(target_arch = "wasm32")))]
mod glyph_cache;
#[cfg(all(feature = "cache", not(target_arch = "wasm32")))]
pub use glyph_cache::clear_glyph_cache;
pub mod glyph_instances;
pub mod glyph_vertex_buffer;
pub mod motion;
mod outline_bind_group;
mod overlap_bind_group;
pub mod profiler;
pub mod rasterizer_pipeline;
pub mod rasterizer_renderrer;
mod screen_bind_group;
mod screen_texture;
mod screen_vertex_buffer;
pub mod shader_art_bind_group;
mod straight_run_simplifier;
mod straighten_outline_builder;
pub mod svg;
pub mod time;
pub mod vector_instances;
mod vector_vertex;
pub mod vector_vertex_buffer;
#[cfg(not(target_arch = "wasm32"))]
pub mod vector_vertex_png_renderer;

pub use straighten_outline_builder::StraightenOutlineBuilder;
pub use vector_vertex::{VectorVertex, VectorVertexBuilder};
