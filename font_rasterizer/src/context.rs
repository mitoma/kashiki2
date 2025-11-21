use std::sync::{Arc, mpsc::Sender};

use font_collector::FontRepository;
use log::warn;
use stroke_parser::Action;

use crate::{
    char_width_calcurator::CharWidthCalculator, color_theme::ColorTheme,
    glyph_vertex_buffer::Direction,
};

pub struct Senders {
    ui_string_sender: Sender<String>,
    ui_svg_sender: Sender<(String, String)>,
    action_queue_sender: Sender<Action>,
    post_action_queue_sender: Sender<Action>,
}

impl Senders {
    pub fn new(
        ui_string_sender: Sender<String>,
        ui_svg_sender: Sender<(String, String)>,
        action_queue_sender: Sender<Action>,
        post_action_queue_sender: Sender<Action>,
    ) -> Self {
        Self {
            ui_string_sender,
            ui_svg_sender,
            action_queue_sender,
            post_action_queue_sender,
        }
    }
}

pub struct StateContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub char_width_calcurator: Arc<CharWidthCalculator>,
    pub color_theme: ColorTheme,
    pub window_size: WindowSize,
    pub global_direction: Direction,
    pub font_repository: FontRepository,
    senders: Senders,
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
        senders: Senders,
    ) -> Self {
        Self {
            device,
            queue,
            char_width_calcurator,
            color_theme,
            window_size,
            global_direction,
            font_repository,
            senders,
        }
    }

    #[inline]
    pub fn register_string(&self, value: String) {
        match self.senders.ui_string_sender.send(value) {
            Ok(_) => {}
            Err(err) => {
                warn!("Failed to send string: {}", err)
            }
        }
    }

    #[inline]
    pub fn register_svg(&self, key: String, svg: String) {
        match self.senders.ui_svg_sender.send((key, svg)) {
            Ok(_) => {}
            Err(err) => {
                warn!("Failed to send SVG: {}", err)
            }
        }
    }

    #[inline]
    pub fn register_action(&self, action: Action) {
        match self.senders.action_queue_sender.send(action) {
            Ok(_) => {}
            Err(err) => {
                warn!("Failed to send action: {}", err)
            }
        }
    }

    #[inline]
    pub fn register_post_action(&self, action: Action) {
        match self.senders.post_action_queue_sender.send(action) {
            Ok(_) => {}
            Err(err) => {
                warn!("Failed to send post action: {}", err)
            }
        }
    }

    #[inline]
    pub fn action_sender(&self) -> Sender<Action> {
        self.senders.action_queue_sender.clone()
    }

    #[inline]
    pub fn post_action_sender(&self) -> Sender<Action> {
        self.senders.post_action_queue_sender.clone()
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
