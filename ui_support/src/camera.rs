use cgmath::{InnerSpace, Point3, Vector3};

use font_rasterizer::context::WindowSize;

use crate::{easing_value::EasingPointN, layout_engine::Model};

#[rustfmt::skip]
const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub struct Camera {
    eye: EasingPointN<3>,
    target: EasingPointN<3>,
    up: EasingPointN<3>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
    eye_quaternion: Option<cgmath::Quaternion<f32>>,
}

impl Camera {
    pub fn basic(window_size: WindowSize) -> Self {
        let Vector3 { x, y, z } = cgmath::Vector3::unit_y();
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

    pub fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(
            self.eye.current().into(),
            self.target.current().into(),
            self.up.current().into(),
        );
        let view = if let Some(eye_quaternion) = self.eye_quaternion {
            cgmath::Matrix4::from(eye_quaternion) * view
        } else {
            view
        };

        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        OPENGL_TO_WGPU_MATRIX * proj * view
    }

    // カメラの位置に依存しないビュー行列を作る
    pub fn build_default_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let default_view = cgmath::Matrix4::look_at_rh(
            (0.0, 0.0, 1.0).into(),
            (0.0, 0.0, 0.0).into(),
            cgmath::Vector3::unit_y(),
        );
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        OPENGL_TO_WGPU_MATRIX * proj * default_view
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
    UpdateEyeQuaternion(Option<cgmath::Quaternion<f32>>),
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
    next_target: Option<cgmath::Point3<f32>>,
    next_eye: Option<cgmath::Point3<f32>>,
    next_eye_quaternion: Option<cgmath::Quaternion<f32>>,
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

    pub fn update_eye_quatanion(
        &self,
        camera: &mut Camera,
        eye_quaternion: Option<cgmath::Quaternion<f32>>,
    ) {
        camera.eye_quaternion = eye_quaternion;
    }

    pub fn update_camera(&self, camera: &mut Camera) {
        let current_up: Vector3<f32> = camera.up.current().into();

        camera.eye_quaternion = self.next_eye_quaternion;
        if let Some(next_target) = self.next_target {
            camera.target.update(next_target.into());
        }
        if let Some(next_eye) = self.next_eye {
            camera.eye.update(next_eye.into());
        }

        let mut current_eye: Point3<f32> = camera.eye.last().into();
        let current_target: Point3<f32> = camera.target.last().into();
        // ターゲットからカメラの座標を引く(カメラから見たターゲットの向き)
        let forward = current_target - current_eye;
        // 向きに対する単位行列
        let forward_norm = forward.normalize();
        // 向きへの距離
        let forward_mag = forward.magnitude();

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
        let forward_mag = forward.magnitude();

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
            let forward =
                Point3::<f32>::from(camera.target.last()) - Point3::<f32>::from(camera.eye.last());
            // 向きへの距離
            forward.magnitude()
        };

        let target_position: Point3<f32> = if adjustment == CameraAdjustment::FitBothAndCentering {
            target.position()
        } else {
            target.focus_position()
        };
        let normal = cgmath::Vector3::<f32>::unit_z();

        // aspect は width / height
        let (w, h) = target.bound();
        // bound にちょっと余裕を持たせる。
        let (w, h) = (w + 2.0, h + 2.0);
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

        let up = target.rotation() * cgmath::Vector3::<f32>::unit_y();
        camera.up.update([up.x, up.y, up.z]);
        camera.target.update(target_position.into());
        camera.eye.update(camera_position.into());
    }
}
