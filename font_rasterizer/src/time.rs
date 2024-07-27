use std::sync::{LazyLock, Mutex};

use cgmath::num_traits::ToPrimitive;
use instant::{Duration, SystemTime};

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
}

impl SystemClock {
    fn new() -> Self {
        let start_time = internal_now_millis();
        Self {
            clock_mode: ClockMode::System,
            start_time,
            current_time: 0,
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
        (duration % u32::MAX as u128).to_u32().unwrap()
    }

    fn increment(&mut self, duration: Duration) {
        match self.clock_mode {
            ClockMode::System => { /* noop */ }
            ClockMode::StepByStep => self.current_time = internal_now_millis() - self.start_time,
            ClockMode::Fixed => self.current_time += duration.as_millis(),
        }
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

pub fn increment_fixed_clock(duration: Duration) {
    CLOCK.lock().unwrap().increment(duration);
}

#[cfg(test)]
mod test {
    use instant::Duration;

    use crate::time::{increment_fixed_clock, set_clock_mode};

    use super::now_millis;

    #[test]
    fn test_time() {
        set_clock_mode(crate::time::ClockMode::System);
        println!("system time:{}", now_millis());
        std::thread::sleep(Duration::from_millis(1000));
        println!("system time:{}", now_millis());
    }

    #[test]
    fn test_fixed_time() {
        set_clock_mode(crate::time::ClockMode::Fixed);
        println!("fixed time:{}", now_millis());
        std::thread::sleep(Duration::from_millis(1000));
        println!("fixed time:{}", now_millis());
        increment_fixed_clock(Duration::from_millis(1234));
        println!("fixed time:{}", now_millis());
    }
}
