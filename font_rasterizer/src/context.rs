use std::sync::Arc;

use font_collector::FontRepository;

use crate::{
    char_width_calcurator::CharWidthCalculator, color_theme::ColorTheme,
    glyph_vertex_buffer::Direction,
};

pub struct StateContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub char_width_calcurator: Arc<CharWidthCalculator>,
    pub color_theme: ColorTheme,
    pub window_size: WindowSize,
    pub global_direction: Direction,
    pub font_repository: FontRepository,
}

impl StateContext {
    // TODO: 引数が多いのはまぁ微妙ではあるが、一旦許容している
    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        char_width_calcurator: Arc<CharWidthCalculator>,
        color_theme: ColorTheme,
        window_size: WindowSize,
        global_direction: Direction,
        font_repository: FontRepository,
    ) -> Self {
        Self {
            device,
            queue,
            char_width_calcurator,
            color_theme,
            window_size,
            global_direction,
            font_repository,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl WindowSize {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn aspect(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
}
