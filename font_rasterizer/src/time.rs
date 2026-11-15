use std::{
    collections::VecDeque,
    sync::{LazyLock, Mutex},
};

use web_time::{Duration, SystemTime};

// FPS 計測に使うスライディングウインドウの幅(ミリ秒)
const FPS_WINDOW_MILLIS: u128 = 1000;

pub enum ClockMode {
    // 時刻取得時に、都度現在時刻を取得するモード
    System,
    // 現在時刻取得時に、固定の時刻を返すモード
    // ただし increment_fixed_clock を呼ぶたびに時刻を現在時刻で再設定するので
    // フレーム時間を全オブジェクトで同一するなどの目的で使う。
    StepByStep,
    // 固定時刻を返す。わりとテスト用ですな。
    Fixed,
}

struct SystemClock {
    clock_mode: ClockMode,
    start_time: u128,
    current_time: u128,
    // 直近 FPS_WINDOW_MILLIS 内のフレームの実時刻(ミリ秒)。ClockMode に関わらず実時刻で記録する。
    frame_times: VecDeque<u128>,
}

impl SystemClock {
    fn new() -> Self {
        let start_time = internal_now_millis();
        Self {
            clock_mode: ClockMode::System,
            start_time,
            current_time: 0,
            frame_times: VecDeque::new(),
        }
    }
}

impl SystemClock {
    fn now_millis(&self) -> u32 {
        let duration: u128 = match self.clock_mode {
            ClockMode::System => internal_now_millis() - self.start_time,
            ClockMode::StepByStep => self.current_time,
            ClockMode::Fixed => self.current_time,
        };
        (duration % u32::MAX as u128) as u32
    }

    fn increment(&mut self, duration: Duration) {
        let now = internal_now_millis();
        self.record_frame(now);
        match self.clock_mode {
            ClockMode::System => { /* noop */ }
            ClockMode::StepByStep => self.current_time = now - self.start_time,
            ClockMode::Fixed => self.current_time += duration.as_millis(),
        }
    }

    fn record_frame(&mut self, now: u128) {
        self.frame_times.push_back(now);
        self.discard_old_frames(now);
    }

    fn discard_old_frames(&mut self, now: u128) {
        while self
            .frame_times
            .front()
            .is_some_and(|t| now.saturating_sub(*t) > FPS_WINDOW_MILLIS)
        {
            self.frame_times.pop_front();
        }
    }

    // 参照時刻 now を基準にウインドウを進めてから計算するので、
    // 描画が止まると値は減衰し、ウインドウ幅を過ぎると 0 になる。
    fn fps(&mut self, now: u128) -> f32 {
        self.discard_old_frames(now);
        if self.frame_times.len() < 2 {
            return 0.0;
        }
        let Some(first) = self.frame_times.front() else {
            return 0.0;
        };
        let elapsed = now.saturating_sub(*first);
        if elapsed == 0 {
            return 0.0;
        }
        (self.frame_times.len() - 1) as f32 * 1000.0 / elapsed as f32
    }
}

fn internal_now_millis() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

static CLOCK: LazyLock<Mutex<SystemClock>> = LazyLock::new(|| Mutex::new(SystemClock::new()));

pub fn set_clock_mode(clock_mode: ClockMode) {
    CLOCK.lock().unwrap().clock_mode = clock_mode;
}

// 初回計測時を 0 としてそれからの経過ミリ秒を返す。
// u32 の範囲を超えたら 0 に巻き戻る(49日ほど)
pub fn now_millis() -> u32 {
    CLOCK.lock().unwrap().now_millis()
}

// 1 フレームごとに呼ばれる前提。フレーム時刻の記録(FPS 計測)も兼ねる。
pub fn increment_fixed_clock(duration: Duration) {
    CLOCK.lock().unwrap().increment(duration);
}

// 現在時刻から遡って直近 1 秒間に increment_fixed_clock が呼ばれた回数から算出した FPS を返す。
// ClockMode に関わらず実時刻で計測する。ウインドウ内のフレームが 2 つ未満の場合は 0.0。
pub fn fps() -> f32 {
    CLOCK.lock().unwrap().fps(internal_now_millis())
}

#[cfg(test)]
mod test {
    use web_time::Duration;

    use super::{ClockMode, SystemClock, internal_now_millis};

    #[test]
    fn test_time() {
        let mut clock = SystemClock::new();
        clock.start_time -= 1234;
        let before = clock.now_millis();
        assert!(before >= 1234);

        clock.increment(Duration::from_millis(5678));
        assert_eq!(clock.current_time, 0);
        assert!(clock.now_millis() >= before);
    }

    #[test]
    fn test_fixed_time() {
        let mut clock = SystemClock::new();
        clock.clock_mode = ClockMode::Fixed;
        clock.current_time = 42;
        assert_eq!(clock.now_millis(), 42);

        clock.start_time -= 1000;
        assert_eq!(clock.now_millis(), 42);
        clock.increment(Duration::from_millis(1234));
        assert_eq!(clock.now_millis(), 1276);
    }

    #[test]
    fn test_step_by_step_time() {
        let mut clock = SystemClock::new();
        clock.clock_mode = ClockMode::StepByStep;
        clock.start_time -= 1234;
        assert_eq!(clock.now_millis(), 0);

        let before = internal_now_millis() - clock.start_time;
        clock.increment(Duration::from_millis(5678));
        let after = internal_now_millis() - clock.start_time;
        assert!((before..=after).contains(&clock.current_time));
        assert_eq!(clock.now_millis(), clock.current_time as u32);
    }

    #[test]
    fn test_fps() {
        let mut clock = SystemClock::new();
        assert_eq!(clock.fps(10_000), 0.0);
        clock.record_frame(10_000);
        assert_eq!(clock.fps(10_000), 0.0);

        // 20ms 間隔 = 50fps
        for i in 1..=100 {
            clock.record_frame(10_000 + i * 20);
        }
        assert_eq!(clock.fps(12_000), 50.0);
        // ウインドウ外の古いフレームは捨てられる
        assert_eq!(clock.frame_times.len(), 51);

        // 描画が止まると参照時刻に応じて値が下がり、ウインドウを過ぎると 0 になる
        assert_eq!(clock.fps(12_500), 25.0);
        assert_eq!(clock.fps(13_001), 0.0);
        assert!(clock.frame_times.is_empty());

        // 長時間フレームが来なかった後は古い記録が破棄される
        clock.record_frame(20_000);
        clock.record_frame(20_100);
        assert_eq!(clock.fps(20_100), 10.0);
        assert_eq!(clock.frame_times.len(), 2);
    }
}
