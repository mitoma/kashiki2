use cgmath::{Point2, Point3};
use instant::Duration;
use nenobi::array::TimeBaseEasingValueN;

pub struct EasingPointN<const N: usize> {
    in_animation: bool,
    v: TimeBaseEasingValueN<f32, N>,
    duration: Duration,
    easing_func: fn(f32) -> f32,
}

impl<const N: usize> EasingPointN<N> {
    pub(crate) fn new(v: [f32; N]) -> Self {
        Self {
            in_animation: true,
            v: TimeBaseEasingValueN::new(v),
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

pub struct EasingPoint3 {
    in_animation: bool,
    v: TimeBaseEasingValueN<f32, 3>,
    duration: Duration,
    easing_func: fn(f32) -> f32,
}

impl EasingPoint3 {
    pub(crate) fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            in_animation: true,
            v: TimeBaseEasingValueN::new([x, y, z]),
            duration: Duration::from_millis(500),
            easing_func: nenobi::functions::sin_out,
        }
    }

    pub(crate) fn current(&self) -> (f32, f32, f32) {
        let [x, y, z] = self.v.current_value();
        (x, y, z)
    }

    pub(crate) fn last(&self) -> (f32, f32, f32) {
        let [x, y, z] = self.v.last_value();
        (x, y, z)
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

    pub(crate) fn update(&mut self, p: cgmath::Point3<f32>) {
        let modify = self.v.update(p.into(), self.duration, self.easing_func);
        self.in_animation = modify;
        self.gc();
    }

    pub(crate) fn add(&mut self, p: cgmath::Point3<f32>) {
        let modify = self.v.add(p.into(), self.duration, self.easing_func);
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

impl From<(f32, f32, f32)> for EasingPoint3 {
    fn from((x, y, z): (f32, f32, f32)) -> Self {
        Self::new(x, y, z)
    }
}

impl From<Point3<f32>> for EasingPoint3 {
    fn from(v: Point3<f32>) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

//  bounce の値に必要そうなので EasingPoint2 も導入する。
// これは EasingPoint3 と合わせて抽象化できるかもしれないが、
// 実装が複雑になるのでいったんベタ書きでの対応とする。
pub struct EasingPoint2 {
    in_animation: bool,
    v: TimeBaseEasingValueN<f32, 2>,
    duration: Duration,
    easing_func: fn(f32) -> f32,
}

impl EasingPoint2 {
    pub(crate) fn new(x: f32, y: f32) -> Self {
        Self {
            in_animation: true,
            v: TimeBaseEasingValueN::new([x, y]),
            duration: Duration::from_millis(500),
            easing_func: nenobi::functions::sin_out,
        }
    }

    pub(crate) fn current(&self) -> (f32, f32) {
        let [x, y] = self.v.current_value();
        (x, y)
    }

    pub(crate) fn last(&self) -> (f32, f32) {
        let [x, y] = self.v.last_value();
        (x, y)
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

    pub(crate) fn update(&mut self, p: cgmath::Point2<f32>) {
        let modify = self.v.update(p.into(), self.duration, self.easing_func);
        self.in_animation = modify;
        self.gc();
    }

    #[allow(dead_code)]
    pub(crate) fn add(&mut self, p: cgmath::Point2<f32>) {
        let modify = self.v.add(p.into(), self.duration, self.easing_func);
        self.in_animation = modify;
        self.gc();
    }

    #[allow(dead_code)]
    pub(crate) fn update_duration_and_easing_func(
        &mut self,
        duration: Duration,
        easing_func: fn(f32) -> f32,
    ) {
        self.duration = duration;
        self.easing_func = easing_func;
    }
}

impl From<(f32, f32)> for EasingPoint2 {
    fn from((x, y): (f32, f32)) -> Self {
        Self::new(x, y)
    }
}

impl From<Point2<f32>> for EasingPoint2 {
    fn from(v: Point2<f32>) -> Self {
        Self::new(v.x, v.y)
    }
}
