use std::sync::Arc;

use winit::dpi::PhysicalSize;

use crate::{char_width_calcurator::CharWidthCalculator, color_theme::ColorTheme};

pub struct StateContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub(crate) char_width_calcurator: Arc<CharWidthCalculator>,
    pub color_theme: ColorTheme,
    pub window_size: WindowSize,
}

#[derive(Debug, Clone, Copy)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl From<PhysicalSize<u32>> for WindowSize {
    fn from(size: PhysicalSize<u32>) -> Self {
        Self {
            width: size.width,
            height: size.height,
        }
    }
}
