use font_rasterizer::context::WindowSize;
use glam::{Mat4, Quat, Vec3};

use crate::{easing_value::EasingPointN, layout_engine::Model};

#[rustfmt::skip]
const OPENGL_TO_WGPU_MATRIX: Mat4 = Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
]);

pub struct Camera {
    eye: EasingPointN<3>,
    target: EasingPointN<3>,
    up: EasingPointN<3>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
    eye_quaternion: Option<Quat>,
}

impl Camera {
    pub fn basic(window_size: WindowSize) -> Self {
        let Vec3 { x, y, z } = Vec3::Y;
        Self::new(
            [0.0, 0.0, 1.0].into(),
            [0.0, 0.0, 0.0].into(),
            [x, y, z].into(),
            window_size.aspect(),
            // fovy は視野角。ここでは45度を指定
            45.0,
            0.1,
            1000.0,
        )
    }

    pub fn new(
        eye: EasingPointN<3>,    // 視点の位置
        target: EasingPointN<3>, // ターゲットの位置
        up: EasingPointN<3>,     // 上を指す単位行列 (x:0, y:1, z:0)
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
            eye_quaternion: None,
        }
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let view = Mat4::look_at_rh(
            self.eye.current().into(),
            self.target.current().into(),
            self.up.current().into(),
        );
        let view = if let Some(eye_quaternion) = self.eye_quaternion {
            Mat4::from_quat(eye_quaternion) * view
        } else {
            view
        };

        let proj = Mat4::perspective_rh(self.fovy.to_radians(), self.aspect, self.znear, self.zfar);
        OPENGL_TO_WGPU_MATRIX * proj * view
    }

    // カメラの位置に依存しないビュー行列を作る
    pub fn build_default_view_projection_matrix(&self) -> Mat4 {
        let default_view =
            Mat4::look_at_rh((0.0, 0.0, 1.0).into(), (0.0, 0.0, 0.0).into(), Vec3::Y);
        let proj = Mat4::perspective_rh(self.fovy.to_radians(), self.aspect, self.znear, self.zfar);
        OPENGL_TO_WGPU_MATRIX * proj * default_view
    }

    pub fn aspect(&self) -> f32 {
        self.aspect
    }

    pub(crate) fn target_and_eye(&self) -> ([f32; 3], [f32; 3]) {
        (self.target.last(), self.eye.last())
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
    CangeTarget(Vec3),
    CangeTargetAndEye(Vec3, Vec3),
    UpdateEyeQuaternion(Option<Quat>),
    None,
}

#[derive(PartialEq)]
pub enum CameraAdjustment {
    // モデルの縦横が画面内に収まるようにする
    FitBoth,
    // モデルの横が画面内に収まるようにする
    FitWidth,
    // モデルの縦が画面内に収まるようにする
    FitHeight,
    // モデルの全体が画面内に収まるようにする
    FitBothAndCentering,
    // 何もしない
    NoCare,
}

pub struct CameraController {
    speed: f32,
    is_up_pressed: bool,
    is_down_pressed: bool,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    next_target: Option<Vec3>,
    next_eye: Option<Vec3>,
    next_eye_quaternion: Option<Quat>,
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
            next_eye_quaternion: None,
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
            CameraOperation::UpdateEyeQuaternion(q) => {
                self.next_eye_quaternion = *q;
            }
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

    pub fn update_camera_aspect(&self, camera: &mut Camera, window_size: WindowSize) {
        camera.aspect = window_size.aspect();
    }

    pub fn update_eye_quatanion(&self, camera: &mut Camera, eye_quaternion: Option<Quat>) {
        camera.eye_quaternion = eye_quaternion;
    }

    pub fn update_camera(&self, camera: &mut Camera) {
        let current_up: Vec3 = camera.up.current().into();

        camera.eye_quaternion = self.next_eye_quaternion;
        if let Some(next_target) = self.next_target {
            camera.target.update(next_target.into());
        }
        if let Some(next_eye) = self.next_eye {
            camera.eye.update(next_eye.into());
        }

        let mut current_eye: Vec3 = camera.eye.last().into();
        let current_target: Vec3 = camera.target.last().into();
        // ターゲットからカメラの座標を引く(カメラから見たターゲットの向き)
        let forward = current_target - current_eye;
        // 向きに対する単位行列
        let forward_norm = forward.normalize();
        // 向きへの距離
        let forward_mag = forward.length();

        // TODO: 近づきすぎないようにしているが、もう少し良い制御したいよ
        if self.is_forward_pressed && forward_mag > self.speed {
            // カメラの位置に向きの単位行列 * 速度分足加える(近づく)
            current_eye += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            // カメラの位置に向きの単位行列 * 速度分足引く(離れる)
            current_eye -= forward_norm * self.speed;
        }

        let right = forward_norm.cross(current_up); // ターゲットへの単位行列と縦軸との外積をとる

        // なぜ再定義が必要？
        let forward = current_target - current_eye;
        let forward_mag = forward.length();

        if self.is_right_pressed {
            // ターゲットから、カメラのほうの少し右を見る単位行列を作り、それに元の距離をかける
            current_eye = current_target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            current_eye = current_target - (forward - right * self.speed).normalize() * forward_mag;
        }

        if self.is_up_pressed {
            current_eye += current_up * self.speed;
        }
        if self.is_down_pressed {
            current_eye -= current_up * self.speed;
        }
        camera.eye.update(current_eye.into());
    }

    pub fn look_at(&self, camera: &mut Camera, target: &dyn Model, adjustment: CameraAdjustment) {
        let forward_mag = {
            let forward = Vec3::from(camera.target.last()) - Vec3::from(camera.eye.last());
            // 向きへの距離
            forward.length()
        };

        let target_position: Vec3 = if adjustment == CameraAdjustment::FitBothAndCentering {
            target.position()
        } else {
            target.focus_position()
        };
        let normal = Vec3::Z;

        // aspect は width / height
        let (w, h) = target.bound();
        // bound にちょっと余裕を持たせる。
        let (w, h) = (w + 3.0, h + 3.0);
        let size = match adjustment {
            // w と h のうち大きい方を使う
            CameraAdjustment::FitBoth | CameraAdjustment::FitBothAndCentering => {
                if w / camera.aspect > h {
                    (w / 2.0) / (camera.fovy.to_radians() / 2.0).tan() / camera.aspect
                } else {
                    (h / 2.0) / (camera.fovy.to_radians() / 2.0).tan()
                }
            }
            CameraAdjustment::FitWidth => {
                (w / 2.0) / (camera.fovy.to_radians() / 2.0).tan() / camera.aspect
            }
            CameraAdjustment::FitHeight => (h / 2.0) / (camera.fovy.to_radians() / 2.0).tan(),
            CameraAdjustment::NoCare => forward_mag,
        };

        let camera_position = target_position + (target.rotation() * normal * (size));

        let up = target.rotation() * Vec3::Y;
        camera.up.update([up.x, up.y, up.z]);
        camera.target.update(target_position.into());
        camera.eye.update(camera_position.into());
    }
}
