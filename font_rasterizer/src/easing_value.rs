use std::sync::OnceLock;

use cgmath::{Point2, Point3};
use instant::Duration;
use nenobi::array::{TimeBaseEasingValueN, TimeBaseEasingValueNFactory};

use crate::time::now_millis;

pub struct EasingPointN<const N: usize> {
    in_animation: bool,
    v: TimeBaseEasingValueN<f32, N>,
    duration: Duration,
    easing_func: fn(f32) -> f32,
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
            in_animation: true,
            v,
            duration: Duration::from_millis(500),
            easing_func: nenobi::functions::sin_out,
        }
    }

    pub(crate) fn current(&self) -> [f32; N] {
        self.v.current_value()
    }

    pub(crate) fn last(&self) -> [f32; N] {
        self.v.last_value()
    }

    pub(crate) fn gc(&mut self) {
        self.v.gc();
    }

    // 実用上 in_animation の最後の判定時に true を返さないと
    // last_value と同一の値の current_value を取りづらいので
    // 最後の一回だけアニメーション中ではなくても true を返す。
    // これは破壊的な処理なので mut になっている。
    pub(crate) fn in_animation(&mut self) -> bool {
        let in_animcation = self.v.in_animation();
        if in_animcation {
            return true;
        }
        if self.in_animation {
            self.in_animation = false;
            return true;
        }
        false
    }

    pub(crate) fn update(&mut self, v: [f32; N]) {
        let modify = self.v.update(v, self.duration, self.easing_func);
        self.in_animation = modify;
        self.gc();
    }

    pub(crate) fn add(&mut self, v: [f32; N]) {
        let modify = self.v.add(v, self.duration, self.easing_func);
        self.in_animation = modify;
        self.gc();
    }

    pub(crate) fn update_duration_and_easing_func(
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
