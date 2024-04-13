use std::sync::Arc;

use cgmath::Point2;
use instant::Duration;
use text_buffer::editor::LineBoundaryProhibitedChars;
use winit::dpi::PhysicalSize;

use crate::{
    char_width_calcurator::CharWidthCalculator,
    color_theme::ColorTheme,
    font_buffer::Direction,
    motion::{MotionDetail, MotionFlags, MotionTarget, MotionType},
};

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

#[allow(dead_code)]
pub struct CpuEasingConfig {
    duration: Duration,
    easing_func: fn(f32) -> f32,
}

pub(crate) struct GpuEasingConfig {
    pub(crate) motion: MotionFlags,
    pub(crate) duration: Duration,
    pub(crate) gain: f32,
}

pub(crate) struct CharEasings {
    pub(crate) add_char: GpuEasingConfig,
    pub(crate) move_char: GpuEasingConfig,
    pub(crate) remove_char: GpuEasingConfig,
    pub(crate) select_char: GpuEasingConfig,
    pub(crate) unselect_char: GpuEasingConfig,
}

impl Default for CharEasings {
    fn default() -> Self {
        Self {
            add_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(
                        crate::motion::EasingFuncType::Back,
                        false,
                    ))
                    .motion_detail(MotionDetail::TO_CURRENT)
                    .motion_target(MotionTarget::MOVE_Y_PLUS | MotionTarget::STRETCH_X_PLUS)
                    .build(),
                duration: Duration::from_millis(500),
                gain: 0.8,
            },
            move_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(
                        crate::motion::EasingFuncType::Sin,
                        false,
                    ))
                    .motion_detail(MotionDetail::TURN_BACK)
                    .motion_target(MotionTarget::MOVE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(300),
                gain: 0.5,
            },
            remove_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(
                        crate::motion::EasingFuncType::Bounce,
                        false,
                    ))
                    .motion_target(MotionTarget::MOVE_Y_MINUS | MotionTarget::STRETCH_X_MINUS)
                    .build(),
                duration: Duration::from_millis(500),
                gain: 0.8,
            },
            select_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(
                        crate::motion::EasingFuncType::Sin,
                        false,
                    ))
                    .motion_target(MotionTarget::ROTATE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(300),
                gain: 1.0,
            },
            unselect_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(
                        crate::motion::EasingFuncType::Sin,
                        false,
                    ))
                    .motion_target(MotionTarget::ROTATE_Y_MINUX)
                    .build(),
                duration: Duration::from_millis(300),
                gain: 1.0,
            },
        }
    }
}

pub struct TextContext {
    pub(crate) direction: Direction,
    pub(crate) row_interval: f32,
    pub(crate) col_interval: f32,
    pub(crate) max_col: usize,
    pub(crate) line_prohibited_chars: LineBoundaryProhibitedChars,
    pub(crate) min_bound: Point2<f32>,
    #[allow(dead_code)]
    pub(crate) position_easing: CpuEasingConfig,
    pub(crate) char_easings: CharEasings,
    pub(crate) color_theme: ColorTheme,
    pub(crate) psychedelic: bool,
}

impl Default for TextContext {
    fn default() -> Self {
        Self {
            direction: Direction::Horizontal,
            row_interval: 1.0,
            col_interval: 0.7,
            max_col: 40,
            line_prohibited_chars: LineBoundaryProhibitedChars::default(),
            min_bound: (10.0, 10.0).into(),
            position_easing: CpuEasingConfig {
                duration: Duration::from_millis(800),
                easing_func: nenobi::functions::sin_in_out,
            },
            char_easings: CharEasings::default(),
            color_theme: ColorTheme::SolarizedDark,
            psychedelic: false,
        }
    }
}
