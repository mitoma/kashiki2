//! WGSL sources used by the font rasterizer pipelines.
//!
//! Keeping these sources available from the crate lets alternative render hosts,
//! such as `bevy_vector_text`, reuse the canonical shader implementation while
//! providing their own wgpu resource and pipeline adapters.

pub const OVERLAP: &str = include_str!("shader/overlap_shader.wgsl");
pub const OUTLINE: &str = include_str!("shader/outline_shader.wgsl");
pub const SCREEN: &str = include_str!("shader/screen_shader.wgsl");
