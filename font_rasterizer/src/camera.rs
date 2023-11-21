use cgmath::{InnerSpace, Point3};
use instant::Duration;
use nenobi::TimeBaseEasingValue;

use crate::layout_engine::Model;

#[rustfmt::skip]
const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

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

    fn current(&self) -> (f32, f32, f32) {
        (
            self.x.current_value(),
            self.y.current_value(),
            self.z.current_value(),
        )
    }

    fn last(&self) -> (f32, f32, f32) {
        (
            self.x.last_value(),
            self.y.last_value(),
            self.z.last_value(),
        )
    }

    fn gc(&mut self) {
        self.x.gc();
        self.y.gc();
        self.z.gc();
    }

    fn update(&mut self, p: cgmath::Point3<f32>) {
        self.x
            .update(p.x, Duration::from_millis(500), nenobi::functions::back_out);
        self.y
            .update(p.y, Duration::from_millis(500), nenobi::functions::back_out);
        self.z
            .update(p.z, Duration::from_millis(500), nenobi::functions::back_out);
        self.gc();
    }
}

impl From<(f32, f32, f32)> for EasingPoint3 {
    fn from((x, y, z): (f32, f32, f32)) -> Self {
        Self::new(x, y, z)
    }
}

pub struct Camera {
    eye: EasingPoint3,
    target: EasingPoint3,
    up: cgmath::Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    pub fn basic(aspect: (u32, u32)) -> Self {
        Self::new(
            (0.0, 0.0, 1.0).into(),
            (0.0, 0.0, 0.0).into(),
            cgmath::Vector3::unit_y(),
            aspect.0 as f32 / aspect.1 as f32,
            // fovy は視野角。ここでは45度を指定
            45.0,
            0.1,
            200.0,
        )
    }

    pub fn new(
        eye: EasingPoint3,        // 視点の位置
        target: EasingPoint3,     // ターゲットの位置
        up: cgmath::Vector3<f32>, // 上を指す単位行列 (x:0, y:1, z:0)
        aspect: f32,
        fovy: f32,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            eye,
            target,
            up,
            aspect,
            fovy,
            znear,
            zfar,
        }
    }

    pub fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(
            self.eye.current().into(),
            self.target.current().into(),
            self.up,
        );
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        OPENGL_TO_WGPU_MATRIX * proj * view
    }

    pub fn aspect(&self) -> f32 {
        self.aspect
    }
}

#[derive(PartialEq)]
pub enum CameraOperation {
    Up,
    Down,
    Left,
    Right,
    Forward,
    Backward,
    CangeTarget(cgmath::Point3<f32>),
    CangeTargetAndEye(cgmath::Point3<f32>, cgmath::Point3<f32>),
    None,
}

pub struct CameraController {
    speed: f32,
    is_up_pressed: bool,
    is_down_pressed: bool,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    next_target: Option<cgmath::Point3<f32>>,
    next_eye: Option<cgmath::Point3<f32>>,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_up_pressed: false,
            is_down_pressed: false,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            next_target: None,
            next_eye: None,
        }
    }

    pub fn process(&mut self, op: &CameraOperation) {
        match op {
            CameraOperation::Up => self.is_up_pressed = true,
            CameraOperation::Down => self.is_down_pressed = true,
            CameraOperation::Right => self.is_right_pressed = true,
            CameraOperation::Left => self.is_left_pressed = true,
            CameraOperation::Forward => self.is_forward_pressed = true,
            CameraOperation::Backward => self.is_backward_pressed = true,
            CameraOperation::CangeTarget(next_target) => self.next_target = Some(*next_target),
            CameraOperation::CangeTargetAndEye(next_target, next_eye) => {
                self.next_target = Some(*next_target);
                self.next_eye = Some(*next_eye);
            }
            CameraOperation::None => {}
        }
    }

    pub fn reset_state(&mut self) {
        self.is_up_pressed = false;
        self.is_down_pressed = false;
        self.is_forward_pressed = false;
        self.is_backward_pressed = false;
        self.is_left_pressed = false;
        self.is_right_pressed = false;
        self.next_eye = None;
        self.next_target = None;
    }

    pub fn update_camera_aspect(&self, camera: &mut Camera, width: u32, height: u32) {
        camera.aspect = width as f32 / height as f32;
    }

    pub fn update_camera(&self, camera: &mut Camera) {
        if let Some(next_target) = self.next_target {
            camera.target.update(next_target);
        }
        if let Some(next_eye) = self.next_eye {
            camera.eye.update(next_eye);
        }

        let mut current_eye: Point3<f32> = camera.eye.last().into();
        let current_target: Point3<f32> = camera.target.last().into();
        // ターゲットからカメラの座標を引く(カメラから見たターゲットの向き)
        let forward = current_target - current_eye;
        // 向きに対する単位行列
        let forward_norm = forward.normalize();
        // 向きへの距離
        let forward_mag = forward.magnitude();

        if self.is_forward_pressed && forward_mag > self.speed {
            // カメラの位置に向きの単位行列 * 速度分足加える(近づく)
            current_eye += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            // カメラの位置に向きの単位行列 * 速度分足引く(離れる)
            current_eye -= forward_norm * self.speed;
        }

        let right = forward_norm.cross(camera.up); // ターゲットへの単位行列と縦軸との外積をとる

        // なぜ再定義が必要？
        let forward = current_target - current_eye;
        let forward_mag = forward.magnitude();

        if self.is_right_pressed {
            // ターゲットから、カメラのほうの少し右を見る単位行列を作り、それに元の距離をかける
            current_eye = current_target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            current_eye = current_target - (forward - right * self.speed).normalize() * forward_mag;
        }

        if self.is_up_pressed {
            current_eye += camera.up * self.speed;
        }
        if self.is_down_pressed {
            current_eye -= camera.up * self.speed;
        }
        camera.eye.update(current_eye);
    }

    pub fn look_at(&self, camera: &mut Camera, target: &dyn Model) {
        let target_position: Point3<f32> = target.position();
        let normal = cgmath::Vector3::<f32>::unit_z();
        let camera_position = target_position + normal;

        camera.target.update(camera_position);
        camera.eye.update(camera_position);
    }
}
