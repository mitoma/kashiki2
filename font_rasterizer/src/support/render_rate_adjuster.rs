use instant::{Duration, Instant};

pub(super) struct RenderRateAdjuster {
    has_focus: bool,
    last_render_time: Instant,
    focused_target_frame_duration: Duration,
    unfocused_target_frame_duration: Duration,
}

impl RenderRateAdjuster {
    pub(super) fn new(focused_target_frame_rate: u32, unfocused_target_frame_rate: u32) -> Self {
        let focused_target_frame_duration =
            instant::Duration::from_secs_f32(1.0 / focused_target_frame_rate as f32);
        let unfocused_target_frame_duration =
            instant::Duration::from_secs_f32(1.0 / unfocused_target_frame_rate as f32);
        Self {
            has_focus: true,
            last_render_time: instant::Instant::now(),
            focused_target_frame_duration,
            unfocused_target_frame_duration,
        }
    }

    pub(super) fn change_focus(&mut self, focused: bool) {
        self.has_focus = focused;
    }

    pub(super) fn skip(&mut self) -> bool {
        let target_frame_duration = if self.has_focus {
            self.focused_target_frame_duration
        } else {
            self.unfocused_target_frame_duration
        };
        if self.last_render_time.elapsed() < target_frame_duration {
            true
        } else {
            self.last_render_time = instant::Instant::now();
            false
        }
    }
}
