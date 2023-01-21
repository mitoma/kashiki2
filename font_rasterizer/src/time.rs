use std::ops::Deref;

use cgmath::num_traits::ToPrimitive;
use instant::SystemTime;
use once_cell::sync::Lazy;

fn internal_now_millis() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

// 初回計測時を 0 としてそれからの経過ミリ秒を返す。
// u32 の範囲を超えたら 0 に巻き戻る(49日ほど)
pub fn now_millis() -> u32 {
    static START_TIME: Lazy<u128> = Lazy::new(internal_now_millis);
    let duration: u128 = internal_now_millis() - START_TIME.deref();
    (duration % u32::MAX as u128).to_u32().unwrap()
}

#[cfg(test)]
mod test {
    use instant::Duration;

    use super::now_millis;

    #[test]
    fn test_time() {
        println!("time:{}", now_millis());
        std::thread::sleep(Duration::from_millis(1000));
        println!("time:{}", now_millis());
    }
}
