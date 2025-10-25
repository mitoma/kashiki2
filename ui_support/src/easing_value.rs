use std::sync::{Mutex, OnceLock};

use cgmath::{Point2, Point3};
use nenobi::array::{TimeBaseEasingValueN, TimeBaseEasingValueNFactory};
use web_time::Duration;

use font_rasterizer::time::now_millis;

pub struct EasingPointN<const N: usize> {
    v: TimeBaseEasingValueN<f32, N>,
    duration: Duration,
    easing_func: fn(f32) -> f32,
    last_evaluation: Mutex<LastEvaluation>,
}

struct LastEvaluation {
    time: i64,
    in_animation: bool,
}

// FIXME このキャストなんだかばかばかしいから直したい
#[inline]
fn now_millis_i64() -> i64 {
    now_millis() as i64
}

impl<const N: usize> EasingPointN<N> {
    pub(crate) fn new(v: [f32; N]) -> Self {
        static FACTORY: OnceLock<TimeBaseEasingValueNFactory> = OnceLock::new();
        let factory = FACTORY.get_or_init(|| TimeBaseEasingValueNFactory::new(now_millis_i64));
        let v = factory.new_value(v);
        Self {
            v,
            duration: Duration::from_millis(500),
            easing_func: nenobi::functions::sin_out,
            last_evaluation: Mutex::new(LastEvaluation {
                time: now_millis_i64(),
                in_animation: true,
            }),
        }
    }

    pub fn current(&self) -> [f32; N] {
        self.v.current_value()
    }

    pub fn last(&self) -> [f32; N] {
        self.v.last_value()
    }

    pub fn gc(&mut self) {
        self.v.gc();
    }

    // 実用上、前回の in_animation の次の判定時は true を返しておかないと
    // last_value と同一の値の current_value を取りづらいので
    // 最後の一回だけアニメーション中ではなくても true を返す。
    pub fn in_animation(&self) -> bool {
        let now = now_millis_i64();
        let mut last_evaluation = self.last_evaluation.lock().unwrap();
        if now == last_evaluation.time {
            return last_evaluation.in_animation;
        }
        let last_in_animation = last_evaluation.in_animation;
        let current_in_animation = self.v.in_animation();
        *last_evaluation = LastEvaluation {
            time: now,
            in_animation: current_in_animation,
        };
        last_in_animation || current_in_animation
    }

    pub fn update(&mut self, v: [f32; N]) {
        let modify = self.v.update(v, self.duration, self.easing_func);
        self.last_evaluation
            .lock()
            .map(|mut last_evaluation| {
                last_evaluation.time = now_millis_i64();
                last_evaluation.in_animation = last_evaluation.in_animation || modify;
            })
            .unwrap();
        self.gc();
    }

    pub fn add(&mut self, v: [f32; N]) {
        let modify = self.v.add(v, self.duration, self.easing_func);
        self.last_evaluation
            .lock()
            .map(|mut last_evaluation| {
                last_evaluation.time = now_millis_i64();
                last_evaluation.in_animation = last_evaluation.in_animation || modify;
            })
            .unwrap();
        self.gc();
    }

    pub fn update_duration_and_easing_func(
        &mut self,
        duration: Duration,
        easing_func: fn(f32) -> f32,
    ) {
        self.duration = duration;
        self.easing_func = easing_func;
    }
}

impl<const N: usize> From<[f32; N]> for EasingPointN<N> {
    fn from(v: [f32; N]) -> Self {
        Self::new(v)
    }
}

impl From<Point3<f32>> for EasingPointN<3> {
    fn from(v: Point3<f32>) -> Self {
        Self::new(v.into())
    }
}

impl From<Point2<f32>> for EasingPointN<2> {
    fn from(v: Point2<f32>) -> Self {
        Self::new(v.into())
    }
}
