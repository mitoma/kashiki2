use instant::Duration;
use nenobi::TimeBaseEasingValue;

pub struct EasingPoint3 {
    in_animation: bool,
    x: TimeBaseEasingValue<f32>,
    y: TimeBaseEasingValue<f32>,
    z: TimeBaseEasingValue<f32>,
    duration: Duration,
    easing_func: fn(f32) -> f32,
}

impl EasingPoint3 {
    pub(crate) fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            in_animation: true,
            x: TimeBaseEasingValue::new(x),
            y: TimeBaseEasingValue::new(y),
            z: TimeBaseEasingValue::new(z),
            duration: Duration::from_millis(500),
            easing_func: nenobi::functions::sin_out,
        }
    }

    pub(crate) fn current(&self) -> (f32, f32, f32) {
        (
            self.x.current_value(),
            self.y.current_value(),
            self.z.current_value(),
        )
    }

    pub(crate) fn last(&self) -> (f32, f32, f32) {
        (
            self.x.last_value(),
            self.y.last_value(),
            self.z.last_value(),
        )
    }

    pub(crate) fn gc(&mut self) {
        self.x.gc();
        self.y.gc();
        self.z.gc();
    }

    // 実用上 in_animation の最後の判定時に true を返さないと
    // last_value と同一の値の current_value を取りづらいので
    // 最後の一回だけアニメーション中ではなくても true を返す。
    // これは破壊的な処理なので mut になっている。
    pub(crate) fn in_animation(&mut self) -> bool {
        let in_animcation = self.x.in_animation() || self.y.in_animation() || self.z.in_animation();
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
        let x_modify = self.x.update(p.x, self.duration, self.easing_func);
        let y_modify = self.y.update(p.y, self.duration, self.easing_func);
        let z_modify = self.z.update(p.z, self.duration, self.easing_func);
        self.in_animation = x_modify || y_modify || z_modify;
        self.gc();
    }

    pub(crate) fn add(&mut self, p: cgmath::Point3<f32>) {
        let x_modify = self.x.add(p.x, self.duration, self.easing_func);
        let y_modify = self.y.add(p.y, self.duration, self.easing_func);
        let z_modify = self.z.add(p.z, self.duration, self.easing_func);
        self.in_animation = x_modify || y_modify || z_modify;
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

//  bounce の値に必要そうなので EasingPoint2 も導入する。
// これは EasingPoint3 と合わせて抽象化できるかもしれないが、
// 実装が複雑になるのでいったんベタ書きでの対応とする。
pub struct EasingPoint2 {
    in_animation: bool,
    x: TimeBaseEasingValue<f32>,
    y: TimeBaseEasingValue<f32>,
    duration: Duration,
    easing_func: fn(f32) -> f32,
}

impl EasingPoint2 {
    pub(crate) fn new(x: f32, y: f32) -> Self {
        Self {
            in_animation: true,
            x: TimeBaseEasingValue::new(x),
            y: TimeBaseEasingValue::new(y),
            duration: Duration::from_millis(500),
            easing_func: nenobi::functions::sin_out,
        }
    }

    pub(crate) fn current(&self) -> (f32, f32) {
        (self.x.current_value(), self.y.current_value())
    }

    pub(crate) fn last(&self) -> (f32, f32) {
        (self.x.last_value(), self.y.last_value())
    }

    pub(crate) fn gc(&mut self) {
        self.x.gc();
        self.y.gc();
    }

    // 実用上 in_animation の最後の判定時に true を返さないと
    // last_value と同一の値の current_value を取りづらいので
    // 最後の一回だけアニメーション中ではなくても true を返す。
    // これは破壊的な処理なので mut になっている。
    pub(crate) fn in_animation(&mut self) -> bool {
        let in_animcation = self.x.in_animation() || self.y.in_animation();
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
        let x_modify = self.x.update(p.x, self.duration, self.easing_func);
        let y_modify = self.y.update(p.y, self.duration, self.easing_func);
        self.in_animation = x_modify || y_modify;
        self.gc();
    }

    #[allow(dead_code)]
    pub(crate) fn add(&mut self, p: cgmath::Point2<f32>) {
        let x_modify = self.x.add(p.x, self.duration, self.easing_func);
        let y_modify = self.y.add(p.y, self.duration, self.easing_func);
        self.in_animation = x_modify || y_modify;
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
