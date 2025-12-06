use log::warn;
use std::sync::{Arc, mpsc::Sender};
use std::time::Duration;

use font_collector::FontRepository;
use font_rasterizer::{
    char_width_calcurator::CharWidthCalculator,
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    glyph_vertex_buffer::Direction,
    motion::{CameraDetail, EasingFuncType, MotionDetail, MotionFlags, MotionTarget, MotionType},
};
use glam::Vec2;
use stroke_parser::Action;
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

/// CharEasings のプリセットを指定するための enum
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharEasingsPreset {
    /// デフォルトのアニメーション設定
    Default,
    /// 動きのない設定
    ZeroMotion,
    /// カメラの影響を受けない設定
    IgnoreCamera,
    /// ポップで弾むような動き (Elastic, Bounce 中心)
    Poppy,
    /// クールで滑らかな動き (Circ, Quad 中心)
    Cool,
    /// エネルギッシュで素早い動き (Back, Expo 中心)
    Energetic,
    /// 優しくゆったりした動き (Sin, Cubic 中心、長めの duration)
    Gentle,
    /// ミニマルで控えめな動き (Quad のみ、短めの duration)
    Minimal,
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
    pub(crate) add_caret: GpuEasingConfig,
    pub(crate) move_caret: GpuEasingConfig,
    pub(crate) remove_caret: GpuEasingConfig,
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
                    .motion_target(MotionTarget::ROTATE_Y_MINUS)
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
            add_caret: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Sin, false))
                    .motion_detail(MotionDetail::TO_CURRENT)
                    .motion_target(MotionTarget::MOVE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(500),
                gain: 0.8,
            },
            move_caret: GpuEasingConfig::default(),
            remove_caret: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Sin, false))
                    .motion_target(MotionTarget::MOVE_Y_MINUS)
                    .build(),
                duration: Duration::from_millis(500),
                gain: 0.8,
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
            add_caret: GpuEasingConfig::default(),
            move_caret: GpuEasingConfig::default(),
            remove_caret: GpuEasingConfig::default(),
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
            add_caret: ignore_camera_config,
            move_caret: ignore_camera_config,
            remove_caret: ignore_camera_config,
        }
    }

    /// ポップで弾むような動きのプリセット。
    ///
    /// Elastic と Bounce を活用した楽しく弾むアニメーション。
    /// カジュアルなUI、ゲーム、子供向けアプリに適しています。
    pub(crate) fn poppy() -> Self {
        Self {
            add_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Elastic, false))
                    .motion_detail(MotionDetail::TO_CURRENT)
                    .motion_target(MotionTarget::MOVE_Y_PLUS | MotionTarget::STRETCH_X_PLUS)
                    .build(),
                duration: Duration::from_millis(600),
                gain: 1.2,
            },
            move_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Bounce, false))
                    .motion_detail(MotionDetail::TURN_BACK)
                    .motion_target(MotionTarget::MOVE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(400),
                gain: 0.8,
            },
            remove_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Elastic, false))
                    .motion_target(MotionTarget::MOVE_Y_MINUS | MotionTarget::ROTATE_Z_PLUS)
                    .build(),
                duration: Duration::from_millis(600),
                gain: 1.2,
            },
            remove_char_mode: RemoveCharMode::Delayed,
            select_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Elastic, false))
                    .motion_target(MotionTarget::ROTATE_Y_PLUS | MotionTarget::STRETCH_X_PLUS)
                    .build(),
                duration: Duration::from_millis(400),
                gain: 1.0,
            },
            unselect_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseIn(EasingFuncType::Elastic, false))
                    .motion_target(MotionTarget::ROTATE_Y_MINUS | MotionTarget::STRETCH_X_MINUS)
                    .build(),
                duration: Duration::from_millis(400),
                gain: 1.0,
            },
            notify_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Bounce, false))
                    .motion_target(MotionTarget::STRETCH_Y_PLUS | MotionTarget::STRETCH_X_PLUS)
                    .motion_detail(MotionDetail::TURN_BACK)
                    .build(),
                duration: Duration::from_millis(600),
                gain: 4.0,
            },
            position_easing: CpuEasingConfig::default(),
            color_easing: CpuEasingConfig::default(),
            scale_easing: CpuEasingConfig::default(),
            motion_gain_easing: CpuEasingConfig::default(),
            add_caret: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Elastic, false))
                    .motion_detail(MotionDetail::TO_CURRENT)
                    .motion_target(MotionTarget::MOVE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(600),
                gain: 1.2,
            },
            move_caret: GpuEasingConfig::default(),
            remove_caret: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Elastic, false))
                    .motion_target(MotionTarget::MOVE_Y_MINUS)
                    .build(),
                duration: Duration::from_millis(600),
                gain: 1.2,
            },
        }
    }

    /// クールで滑らかな動きのプリセット。
    ///
    /// Circ と Quad を活用した洗練された滑らかなアニメーション。
    /// ビジネスアプリ、プロフェッショナルツール、ダッシュボードに適しています。
    pub(crate) fn cool() -> Self {
        Self {
            add_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Circ, false))
                    .motion_detail(MotionDetail::TO_CURRENT)
                    .motion_target(MotionTarget::MOVE_X_PLUS)
                    .build(),
                duration: Duration::from_millis(400),
                gain: 0.6,
            },
            move_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Quad, false))
                    .motion_target(MotionTarget::MOVE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(250),
                gain: 0.3,
            },
            remove_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseIn(EasingFuncType::Circ, false))
                    .motion_target(MotionTarget::MOVE_X_MINUS | MotionTarget::STRETCH_X_MINUS)
                    .build(),
                duration: Duration::from_millis(400),
                gain: 0.6,
            },
            remove_char_mode: RemoveCharMode::Delayed,
            select_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Circ, false))
                    .motion_target(MotionTarget::ROTATE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(300),
                gain: 0.8,
            },
            unselect_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Circ, false))
                    .motion_target(MotionTarget::ROTATE_Y_MINUS)
                    .build(),
                duration: Duration::from_millis(300),
                gain: 0.8,
            },
            notify_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Quad, false))
                    .motion_target(MotionTarget::STRETCH_X_PLUS)
                    .motion_detail(MotionDetail::TURN_BACK)
                    .build(),
                duration: Duration::from_millis(400),
                gain: 2.0,
            },
            position_easing: CpuEasingConfig::default(),
            color_easing: CpuEasingConfig::default(),
            scale_easing: CpuEasingConfig::default(),
            motion_gain_easing: CpuEasingConfig::default(),
            add_caret: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Circ, false))
                    .motion_detail(MotionDetail::TO_CURRENT)
                    .motion_target(MotionTarget::MOVE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(400),
                gain: 0.6,
            },
            move_caret: GpuEasingConfig::default(),
            remove_caret: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseIn(EasingFuncType::Circ, false))
                    .motion_target(MotionTarget::MOVE_Y_MINUS)
                    .build(),
                duration: Duration::from_millis(400),
                gain: 0.6,
            },
        }
    }

    /// エネルギッシュで素早い動きのプリセット。
    ///
    /// Back と Expo を活用したダイナミックで力強いアニメーション。
    /// アクション重視のUI、通知、アラートに適しています。
    pub(crate) fn energetic() -> Self {
        Self {
            add_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Back, false))
                    .motion_detail(MotionDetail::TO_CURRENT)
                    .motion_target(MotionTarget::MOVE_Y_PLUS | MotionTarget::ROTATE_Z_MINUS)
                    .build(),
                duration: Duration::from_millis(350),
                gain: 1.0,
            },
            move_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Expo, false))
                    .motion_detail(MotionDetail::TURN_BACK)
                    .motion_target(MotionTarget::MOVE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(200),
                gain: 0.7,
            },
            remove_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseIn(EasingFuncType::Expo, false))
                    .motion_target(
                        MotionTarget::MOVE_Y_MINUS
                            | MotionTarget::ROTATE_Z_PLUS
                            | MotionTarget::STRETCH_X_MINUS,
                    )
                    .build(),
                duration: Duration::from_millis(300),
                gain: 1.0,
            },
            remove_char_mode: RemoveCharMode::Delayed,
            select_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Back, false))
                    .motion_target(MotionTarget::ROTATE_Y_PLUS | MotionTarget::STRETCH_X_PLUS)
                    .build(),
                duration: Duration::from_millis(250),
                gain: 1.2,
            },
            unselect_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseIn(EasingFuncType::Back, false))
                    .motion_target(MotionTarget::ROTATE_Y_MINUS | MotionTarget::STRETCH_X_MINUS)
                    .build(),
                duration: Duration::from_millis(250),
                gain: 1.2,
            },
            notify_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Expo, false))
                    .motion_target(
                        MotionTarget::STRETCH_Y_PLUS
                            | MotionTarget::STRETCH_X_PLUS
                            | MotionTarget::ROTATE_Z_PLUS,
                    )
                    .motion_detail(MotionDetail::TURN_BACK)
                    .build(),
                duration: Duration::from_millis(400),
                gain: 3.5,
            },
            position_easing: CpuEasingConfig::default(),
            color_easing: CpuEasingConfig::default(),
            scale_easing: CpuEasingConfig::default(),
            motion_gain_easing: CpuEasingConfig::default(),
            add_caret: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Back, false))
                    .motion_detail(MotionDetail::TO_CURRENT)
                    .motion_target(MotionTarget::MOVE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(350),
                gain: 1.0,
            },
            move_caret: GpuEasingConfig::default(),
            remove_caret: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseIn(EasingFuncType::Expo, false))
                    .motion_target(MotionTarget::MOVE_Y_MINUS)
                    .build(),
                duration: Duration::from_millis(300),
                gain: 1.0,
            },
        }
    }

    /// 優しくゆったりした動きのプリセット。
    ///
    /// Sin と Cubic を活用した柔らかくリラックスした長めのアニメーション。
    /// リーディングアプリ、瞑想アプリ、リラックス系コンテンツに適しています。
    pub(crate) fn gentle() -> Self {
        Self {
            add_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Sin, false))
                    .motion_detail(MotionDetail::TO_CURRENT)
                    .motion_target(MotionTarget::MOVE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(800),
                gain: 0.5,
            },
            move_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Cubic, false))
                    .motion_target(MotionTarget::MOVE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(600),
                gain: 0.3,
            },
            remove_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseIn(EasingFuncType::Sin, false))
                    .motion_target(MotionTarget::MOVE_Y_MINUS | MotionTarget::STRETCH_X_MINUS)
                    .build(),
                duration: Duration::from_millis(800),
                gain: 0.5,
            },
            remove_char_mode: RemoveCharMode::Delayed,
            select_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Cubic, false))
                    .motion_target(MotionTarget::ROTATE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(500),
                gain: 0.7,
            },
            unselect_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Cubic, false))
                    .motion_target(MotionTarget::ROTATE_Y_MINUS)
                    .build(),
                duration: Duration::from_millis(500),
                gain: 0.7,
            },
            notify_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Sin, false))
                    .motion_target(MotionTarget::STRETCH_Y_PLUS | MotionTarget::STRETCH_X_PLUS)
                    .motion_detail(MotionDetail::TURN_BACK)
                    .build(),
                duration: Duration::from_millis(700),
                gain: 2.0,
            },
            position_easing: CpuEasingConfig {
                duration: Duration::from_millis(800),
                easing_func: nenobi::functions::sin_in_out,
            },
            color_easing: CpuEasingConfig {
                duration: Duration::from_millis(800),
                easing_func: nenobi::functions::sin_in_out,
            },
            scale_easing: CpuEasingConfig {
                duration: Duration::from_millis(800),
                easing_func: nenobi::functions::sin_in_out,
            },
            motion_gain_easing: CpuEasingConfig {
                duration: Duration::from_millis(800),
                easing_func: nenobi::functions::sin_in_out,
            },
            add_caret: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Sin, false))
                    .motion_detail(MotionDetail::TO_CURRENT)
                    .motion_target(MotionTarget::MOVE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(800),
                gain: 0.5,
            },
            move_caret: GpuEasingConfig::default(),
            remove_caret: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseIn(EasingFuncType::Sin, false))
                    .motion_target(MotionTarget::MOVE_Y_MINUS)
                    .build(),
                duration: Duration::from_millis(800),
                gain: 0.5,
            },
        }
    }

    /// ミニマルで控えめな動きのプリセット。
    ///
    /// Quad のみを使用した短時間で小さな動き。
    /// データ重視のアプリ、コンソール、ターミナル風UIに適しています。
    pub(crate) fn minimal() -> Self {
        Self {
            add_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Quad, false))
                    .motion_detail(MotionDetail::TO_CURRENT)
                    .motion_target(MotionTarget::MOVE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(200),
                gain: 0.3,
            },
            move_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Quad, false))
                    .motion_target(MotionTarget::MOVE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(150),
                gain: 0.2,
            },
            remove_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseIn(EasingFuncType::Quad, false))
                    .motion_target(MotionTarget::MOVE_Y_MINUS)
                    .build(),
                duration: Duration::from_millis(200),
                gain: 0.3,
            },
            remove_char_mode: RemoveCharMode::Delayed,
            select_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Quad, false))
                    .motion_target(MotionTarget::ROTATE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(200),
                gain: 0.5,
            },
            unselect_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Quad, false))
                    .motion_target(MotionTarget::ROTATE_Y_MINUS)
                    .build(),
                duration: Duration::from_millis(200),
                gain: 0.5,
            },
            notify_char: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseInOut(EasingFuncType::Quad, false))
                    .motion_target(MotionTarget::STRETCH_X_PLUS)
                    .motion_detail(MotionDetail::TURN_BACK)
                    .build(),
                duration: Duration::from_millis(250),
                gain: 1.5,
            },
            position_easing: CpuEasingConfig {
                duration: Duration::from_millis(200),
                easing_func: nenobi::functions::quad_in_out,
            },
            color_easing: CpuEasingConfig {
                duration: Duration::from_millis(200),
                easing_func: nenobi::functions::quad_in_out,
            },
            scale_easing: CpuEasingConfig {
                duration: Duration::from_millis(200),
                easing_func: nenobi::functions::quad_in_out,
            },
            motion_gain_easing: CpuEasingConfig {
                duration: Duration::from_millis(200),
                easing_func: nenobi::functions::quad_in_out,
            },
            add_caret: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseOut(EasingFuncType::Quad, false))
                    .motion_detail(MotionDetail::TO_CURRENT)
                    .motion_target(MotionTarget::MOVE_Y_PLUS)
                    .build(),
                duration: Duration::from_millis(200),
                gain: 0.3,
            },
            move_caret: GpuEasingConfig::default(),
            remove_caret: GpuEasingConfig {
                motion: MotionFlags::builder()
                    .motion_type(MotionType::EaseIn(EasingFuncType::Quad, false))
                    .motion_target(MotionTarget::MOVE_Y_MINUS)
                    .build(),
                duration: Duration::from_millis(200),
                gain: 0.3,
            },
        }
    }

    /// プリセットから CharEasings を生成する
    pub fn from_preset(preset: CharEasingsPreset) -> Self {
        match preset {
            CharEasingsPreset::Default => Self::default(),
            CharEasingsPreset::ZeroMotion => Self::zero_motion(),
            CharEasingsPreset::IgnoreCamera => Self::ignore_camera(),
            CharEasingsPreset::Poppy => Self::poppy(),
            CharEasingsPreset::Cool => Self::cool(),
            CharEasingsPreset::Energetic => Self::energetic(),
            CharEasingsPreset::Gentle => Self::gentle(),
            CharEasingsPreset::Minimal => Self::minimal(),
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
    pub(crate) min_bound: Vec2,
    pub(crate) char_easings: CharEasings,
    pub(crate) color_theme: ColorTheme,
    pub(crate) psychedelic: bool,
    pub(crate) hyde_caret: bool,
    pub(crate) highlight_mode: HighlightMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HighlightMode {
    None,
    Markdown,
    Language(String),
}

const TEXT_CONTEXT_ZERO_BOUND: Vec2 = Vec2::new(0.0, 0.0);
const TEXT_CONTEXT_DEFAULT_BOUND: Vec2 = Vec2::new(10.0, 5.0);

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
            highlight_mode: HighlightMode::None,
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

    #[inline]
    pub fn with_char_easings_preset(mut self, preset: CharEasingsPreset) -> Self {
        self.char_easings = CharEasings::from_preset(preset);
        self
    }

    #[inline]
    pub fn set_char_easings_preset(&mut self, preset: CharEasingsPreset) {
        self.char_easings = CharEasings::from_preset(preset);
    }

    pub fn instance_scale(&self) -> [f32; 2] {
        match self.direction {
            Direction::Horizontal => [self.col_scale, self.row_scale],
            Direction::Vertical => [self.row_scale, self.col_scale],
        }
    }

    pub fn toggle_min_bound(&mut self) {
        if self.min_bound == TEXT_CONTEXT_ZERO_BOUND {
            self.min_bound = TEXT_CONTEXT_DEFAULT_BOUND;
        } else {
            self.min_bound = TEXT_CONTEXT_ZERO_BOUND;
        }
    }
}

/// UI レイヤー向けの sender をまとめた構造体。
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

/// UI レイヤー用のコンテキスト。
/// StateContext をラップし、UI 固有の機能を提供する。
pub struct UiContext {
    state_context: StateContext,
    senders: Senders,
}

impl UiContext {
    #[inline]
    pub fn new(state_context: StateContext, senders: Senders) -> Self {
        Self {
            state_context,
            senders,
        }
    }

    // StateContext のフィールドへのアクセサ

    #[inline]
    pub fn device(&self) -> &wgpu::Device {
        &self.state_context.device
    }

    #[inline]
    pub fn queue(&self) -> &wgpu::Queue {
        &self.state_context.queue
    }

    #[inline]
    pub fn char_width_calcurator(&self) -> &Arc<CharWidthCalculator> {
        &self.state_context.char_width_calcurator
    }

    #[inline]
    pub fn color_theme(&self) -> &ColorTheme {
        &self.state_context.color_theme
    }

    #[inline]
    pub fn window_size(&self) -> WindowSize {
        self.state_context.window_size
    }

    #[inline]
    pub fn global_direction(&self) -> Direction {
        self.state_context.global_direction
    }

    #[inline]
    pub fn set_global_direction(&mut self, direction: Direction) {
        self.state_context.global_direction = direction;
    }

    #[inline]
    pub fn font_repository(&self) -> &FontRepository {
        &self.state_context.font_repository
    }

    // Senders の移譲
    #[inline]
    pub fn register_string(&self, value: String) {
        match self.senders.ui_string_sender.send(value) {
            Ok(_) => {}
            Err(err) => warn!("Failed to send string: {}", err),
        }
    }

    #[inline]
    pub fn register_svg(&self, key: String, svg: String) {
        match self.senders.ui_svg_sender.send((key, svg)) {
            Ok(_) => {}
            Err(err) => warn!("Failed to send SVG: {}", err),
        }
    }

    #[inline]
    pub fn register_action(&self, action: Action) {
        match self.senders.action_queue_sender.send(action) {
            Ok(_) => {}
            Err(err) => warn!("Failed to send action: {}", err),
        }
    }

    #[inline]
    pub fn register_post_action(&self, action: Action) {
        match self.senders.post_action_queue_sender.send(action) {
            Ok(_) => {}
            Err(err) => warn!("Failed to send post action: {}", err),
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

    #[inline]
    pub(crate) fn state_context_mut(&mut self) -> &mut StateContext {
        &mut self.state_context
    }
}
