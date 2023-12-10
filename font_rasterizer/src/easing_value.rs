use instant::Duration;
use nenobi::TimeBaseEasingValue;

pub struct EasingPoint3 {
    x: TimeBaseEasingValue<f32>,
    y: TimeBaseEasingValue<f32>,
    z: TimeBaseEasingValue<f32>,
}

impl EasingPoint3 {
    pub(crate) fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            x: TimeBaseEasingValue::new(x),
            y: TimeBaseEasingValue::new(y),
            z: TimeBaseEasingValue::new(z),
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

    pub(crate) fn in_animation(&self) -> bool {
        self.x.in_animation() || self.y.in_animation() || self.z.in_animation()
    }

    pub(crate) fn update(&mut self, p: cgmath::Point3<f32>) {
        self.x
            .update(p.x, Duration::from_millis(500), nenobi::functions::sin_out);
        self.y
            .update(p.y, Duration::from_millis(500), nenobi::functions::sin_out);
        self.z
            .update(p.z, Duration::from_millis(500), nenobi::functions::sin_out);
        self.gc();
    }

    pub(crate) fn add(&mut self, p: cgmath::Point3<f32>) {
        self.x
            .add(p.x, Duration::from_millis(500), nenobi::functions::sin_out);
        self.y
            .add(p.y, Duration::from_millis(500), nenobi::functions::sin_out);
        self.z
            .add(p.z, Duration::from_millis(500), nenobi::functions::sin_out);
        self.gc();
    }
}

impl From<(f32, f32, f32)> for EasingPoint3 {
    fn from((x, y, z): (f32, f32, f32)) -> Self {
        Self::new(x, y, z)
    }
}
