use std::time::Duration;

use cgmath::Point2;
use font_rasterizer::{
    color_theme::ColorTheme,
    font_buffer::Direction,
    motion::{CameraDetail, EasingFuncType, MotionDetail, MotionFlags, MotionTarget, MotionType},
};
use text_buffer::editor::LineBoundaryProhibitedChars;

pub(crate) struct CpuEasingConfig {
    pub(crate) duration: Duration,
    pub(crate) easing_func: fn(f32) -> f32,
}

impl Default for CpuEasingConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_millis(500),
            easing_func: nenobi::functions::sin_out,
        }
    }
}

impl CpuEasingConfig {
    pub(crate) fn zero_motion() -> Self {
        Self {
            duration: Duration::ZERO,
            easing_func: nenobi::functions::liner,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct GpuEasingConfig {
    pub(crate) motion: MotionFlags,
    pub(crate) duration: Duration,
    pub(crate) gain: f32,
}

impl Default for GpuEasingConfig {
    fn default() -> Self {
        Self {
            motion: MotionFlags::default(),
            duration: Duration::ZERO,
            gain: 0.0,
        }
    }
}

impl GpuEasingConfig {
    pub fn fadein() -> Self {
        Self {
            motion: MotionFlags::builder()
                .motion_detail(MotionDetail::TO_CURRENT)
                .motion_target(MotionTarget::STRETCH_X_MINUS | MotionTarget::STRETCH_Y_MINUS)
                .motion_type(MotionType::EaseInOut(EasingFuncType::Quad, false))
                .build(),
            duration: Duration::from_millis(300),
            gain: 1.0,
        }
    }
    pub fn fadeout() -> Self {
        Self {
            motion: MotionFlags::builder()
                .motion_target(MotionTarget::STRETCH_X_MINUS | MotionTarget::STRETCH_Y_MINUS)
                .motion_type(MotionType::EaseInOut(EasingFuncType::Quad, false))
                .build(),
            duration: Duration::from_millis(300),
            gain: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RemoveCharMode {
    Immediate,
    Delayed,
}

pub(crate) struct CharEasings {
    pub(crate) add_char: GpuEasingConfig,
    pub(crate) move_char: GpuEasingConfig,
    pub(crate) remove_char: GpuEasingConfig,
    pub(crate) remove_char_mode: RemoveCharMode,
    pub(crate) select_char: GpuEasingConfig,
    pub(crate) unselect_char: GpuEasingConfig,
    pub(crate) notify_char: GpuEasingConfig,
    pub(crate) position_easing: CpuEasingConfig,
    pub(crate) color_easing: CpuEasingConfig,
    pub(crate) scale_easing: CpuEasingConfig,
    pub(crate) motion_gain_easing: CpuEasingConfig,
}

impl Default for CharEasings {
    fn default() -> Self {
        Self {
            add_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Back, false))
                    .motion_detail(MotionDetail::TO_CURRENT)
                    .motion_target(MotionTarget::MOVE_Y_PLUS | MotionTarget::STRETCH_X_PLUS)
                    .build(),
                duration: Duration::from_millis(500),
                gain: 0.8,
            },
            move_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Sin, false))
                    .motion_detail(MotionDetail::TURN_BACK)
                    .motion_target(MotionTarget::MOVE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(300),
                gain: 0.5,
            },
            remove_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Bounce, false))
                    .motion_target(MotionTarget::MOVE_Y_MINUS | MotionTarget::STRETCH_X_MINUS)
                    .build(),
                duration: Duration::from_millis(500),
                gain: 0.8,
            },
            remove_char_mode: RemoveCharMode::Delayed,
            select_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Sin, false))
                    .motion_target(MotionTarget::ROTATE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(300),
                gain: 1.0,
            },
            unselect_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Sin, false))
                    .motion_target(MotionTarget::ROTATE_Y_MINUX)
                    .build(),
                duration: Duration::from_millis(300),
                gain: 1.0,
            },
            notify_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Cubic, false))
                    .motion_target(MotionTarget::STRETCH_Y_PLUS | MotionTarget::STRETCH_X_PLUS)
                    .motion_detail(MotionDetail::TURN_BACK)
                    .build(),
                duration: Duration::from_millis(500),
                gain: 3.0,
            },
            position_easing: CpuEasingConfig {
                duration: Duration::from_millis(500),
                easing_func: nenobi::functions::sin_in_out,
            },
            color_easing: CpuEasingConfig {
                duration: Duration::from_millis(500),
                easing_func: nenobi::functions::sin_in_out,
            },
            scale_easing: CpuEasingConfig {
                duration: Duration::from_millis(500),
                easing_func: nenobi::functions::sin_in_out,
            },
            motion_gain_easing: CpuEasingConfig {
                duration: Duration::from_millis(500),
                easing_func: nenobi::functions::sin_in_out,
            },
        }
    }
}

impl CharEasings {
    // 動きが何もない状態の設定を返す
    #[allow(dead_code)]
    pub(crate) fn zero_motion() -> Self {
        Self {
            add_char: GpuEasingConfig::default(),
            move_char: GpuEasingConfig::default(),
            remove_char: GpuEasingConfig::default(),
            remove_char_mode: RemoveCharMode::Immediate,
            select_char: GpuEasingConfig::default(),
            unselect_char: GpuEasingConfig::default(),
            notify_char: GpuEasingConfig::default(),
            position_easing: CpuEasingConfig::default(),
            color_easing: CpuEasingConfig::default(),
            scale_easing: CpuEasingConfig::default(),
            motion_gain_easing: CpuEasingConfig::default(),
        }
    }

    pub(crate) fn ignore_camera() -> Self {
        let ignore_camera_config = GpuEasingConfig {
            motion: MotionFlags::builder()
                .camera_detail(CameraDetail::IGNORE_CAMERA)
                .build(),
            duration: Duration::ZERO,
            gain: 0.0,
        };
        Self {
            add_char: ignore_camera_config,
            move_char: ignore_camera_config,
            remove_char: ignore_camera_config,
            remove_char_mode: RemoveCharMode::Immediate,
            select_char: ignore_camera_config,
            unselect_char: ignore_camera_config,
            notify_char: ignore_camera_config,
            position_easing: CpuEasingConfig::zero_motion(),
            color_easing: CpuEasingConfig::zero_motion(),
            scale_easing: CpuEasingConfig::zero_motion(),
            motion_gain_easing: CpuEasingConfig::zero_motion(),
        }
    }
}

pub struct TextContext {
    pub(crate) direction: Direction,
    pub(crate) row_interval: f32,
    pub(crate) col_interval: f32,
    pub(crate) row_scale: f32,
    pub(crate) col_scale: f32,
    pub(crate) max_col: usize,
    pub(crate) line_prohibited_chars: LineBoundaryProhibitedChars,
    pub(crate) min_bound: Point2<f32>,
    pub(crate) char_easings: CharEasings,
    pub(crate) color_theme: ColorTheme,
    pub(crate) psychedelic: bool,
    pub(crate) hyde_caret: bool,
}

impl Default for TextContext {
    fn default() -> Self {
        // 読みやすい文章の目安として一行日本語30文字程度、
        // 行間を1.5倍、文字間を1.0倍をデフォルトとして設定する。
        Self {
            direction: Direction::Horizontal,
            row_interval: 1.5,
            col_interval: 1.0,
            row_scale: 1.0,
            col_scale: 1.0,
            max_col: 60,
            line_prohibited_chars: LineBoundaryProhibitedChars::default(),
            min_bound: (10.0, 5.0).into(),
            char_easings: CharEasings::default(),
            color_theme: ColorTheme::SolarizedDark,
            psychedelic: false,
            hyde_caret: false,
        }
    }
}

impl TextContext {
    #[inline]
    pub fn with_row_interval(mut self, row_interval: f32) -> Self {
        self.row_interval = row_interval;
        self
    }

    #[inline]
    pub fn with_col_interval(mut self, col_interval: f32) -> Self {
        self.col_interval = col_interval;
        self
    }

    #[inline]
    pub fn with_row_scale(mut self, row_scale: f32) -> Self {
        self.row_scale = row_scale;
        self
    }

    #[inline]
    pub fn with_col_scale(mut self, col_scale: f32) -> Self {
        self.col_scale = col_scale;
        self
    }

    #[inline]
    pub fn with_max_col(mut self, max_col: usize) -> Self {
        self.max_col = max_col;
        self
    }

    pub fn instance_scale(&self) -> [f32; 2] {
        match self.direction {
            Direction::Horizontal => [self.col_scale, self.row_scale],
            Direction::Vertical => [self.row_scale, self.col_scale],
        }
    }
}
