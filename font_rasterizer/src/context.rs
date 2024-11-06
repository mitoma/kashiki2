use std::sync::{mpsc::Sender, Arc};

use stroke_parser::Action;
use winit::dpi::PhysicalSize;

use crate::{
    char_width_calcurator::CharWidthCalculator, color_theme::ColorTheme, font_buffer::Direction,
};

pub struct StateContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub char_width_calcurator: Arc<CharWidthCalculator>,
    pub color_theme: ColorTheme,
    pub window_size: WindowSize,
    pub ui_string_sender: Sender<String>,
    pub action_queue_sender: Sender<Action>,
    pub post_action_queue_sender: Sender<Action>,
    pub global_direction: Direction,
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

impl From<PhysicalSize<u32>> for WindowSize {
    fn from(size: PhysicalSize<u32>) -> Self {
        Self {
            width: size.width,
            height: size.height,
        }
    }
}
