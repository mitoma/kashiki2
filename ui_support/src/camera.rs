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
        let normal = Vec3::Z;

        // aspect は width / height
        let (w, h) = target.bound();
        // bound にちょっと余裕を持たせる。
        let (w, h) = (w * 1.1, h * 1.1);
        let tan_half_fovy = (camera.fovy.to_radians() / 2.0).tan();
        let aspect = camera.aspect.max(f32::EPSILON);
        let size = match adjustment {
            // w と h のうち大きい方を使う
            CameraAdjustment::FitBoth | CameraAdjustment::FitBothAndCentering => {
                if w / aspect > h {
                    (w / 2.0) / tan_half_fovy / aspect
                } else {
                    (h / 2.0) / tan_half_fovy
                }
            }
            CameraAdjustment::FitWidth => (w / 2.0) / tan_half_fovy / aspect,
            CameraAdjustment::FitHeight => (h / 2.0) / tan_half_fovy,
            CameraAdjustment::NoCare => forward_mag,
        };

        let target_position: Vec3 = {
            // A-2改善: X/Y軸ごとに連続補間しつつ、可能なら片側エッジを画面内に残す。
            let visible_half_height = size * tan_half_fovy;
            let visible_half_width = visible_half_height * aspect;
            let width_fit_ratio =
                (visible_half_width / (w / 2.0).max(f32::EPSILON)).clamp(0.0, 1.0);
            let height_fit_ratio =
                (visible_half_height / (h / 2.0).max(f32::EPSILON)).clamp(0.0, 1.0);

            let last_position = target.last_position();
            let focus_position = target.focus_position();

            let calc_axis = |position_axis: f32,
                             focus_axis: f32,
                             half_bound: f32,
                             visible_half: f32,
                             fit_ratio: f32|
             -> (f32, f32) {
                if fit_ratio >= 1.0 || half_bound <= f32::EPSILON {
                    return (position_axis, 0.0);
                }

                let overflow_weight = 1.0 - fit_ratio;
                let blended = position_axis + (focus_axis - position_axis) * overflow_weight;

                let left_edge = position_axis - half_bound;
                let right_edge = position_axis + half_bound;
                let left_anchor = left_edge + visible_half;
                let right_anchor = right_edge - visible_half;

                let can_include_left = (focus_axis - left_anchor).abs() <= visible_half;
                let can_include_right = (focus_axis - right_anchor).abs() <= visible_half;

                let boundary_anchor = if focus_axis >= position_axis {
                    if can_include_right {
                        right_anchor
                    } else if can_include_left {
                        left_anchor
                    } else {
                        focus_axis
                    }
                } else if can_include_left {
                    left_anchor
                } else if can_include_right {
                    right_anchor
                } else {
                    focus_axis
                };

                let mut axis = blended + (boundary_anchor - blended) * overflow_weight;

                // focus は常に画面内に含める。
                let focus_min = focus_axis - visible_half;
                let focus_max = focus_axis + visible_half;
                axis = axis.clamp(focus_min.min(focus_max), focus_min.max(focus_max));

                // 位置はpositionとfocusの間を連続的に遷移させる。
                let between_min = position_axis.min(focus_axis);
                let between_max = position_axis.max(focus_axis);
                axis = axis.clamp(between_min, between_max);

                (axis, overflow_weight)
            };

            let (x, focus_weight_x) = calc_axis(
                last_position.x,
                focus_position.x,
                w / 2.0,
                visible_half_width,
                width_fit_ratio,
            );
            let (y, focus_weight_y) = calc_axis(
                last_position.y,
                focus_position.y,
                h / 2.0,
                visible_half_height,
                height_fit_ratio,
            );
            let focus_weight_z = focus_weight_x.max(focus_weight_y);

            Vec3::new(
                x,
                y,
                last_position.z + (focus_position.z - last_position.z) * focus_weight_z,
            )
        };

        let camera_position = target_position + (target.rotation() * normal * (size));

        let up = target.rotation() * Vec3::Y;
        camera.up.update([up.x, up.y, up.z]);
        camera.target.update(target_position.into());
        camera.eye.update(camera_position.into());
    }
}
