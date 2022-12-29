use cgmath::InnerSpace;
use winit::event::*;

#[rustfmt::skip]
const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub struct Camera {
    eye: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    pub fn new(
        eye: cgmath::Point3<f32>,    // 視点の位置
        target: cgmath::Point3<f32>, // ターゲットの位置
        up: cgmath::Vector3<f32>,    // 上を指す単位行列 (x:0, y:1, z:0)
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
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        OPENGL_TO_WGPU_MATRIX * proj * view
    }
}

pub enum CameraOperation {
    Up,
    Down,
    Left,
    Right,
    Forward,
    Backward,
}

pub struct CameraController {
    speed: f32,
    is_up_pressed: bool,
    is_down_pressed: bool,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
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
        }
    }

    pub fn reset_state(&mut self) {
        self.is_up_pressed = false;
        self.is_down_pressed = false;
        self.is_forward_pressed = false;
        self.is_backward_pressed = false;
        self.is_left_pressed = false;
        self.is_right_pressed = false;
    }

    pub fn process_event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    VirtualKeyCode::Q => {
                        self.is_up_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::E => {
                        self.is_down_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::W | VirtualKeyCode::Up => {
                        self.is_forward_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::A | VirtualKeyCode::Left => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::S | VirtualKeyCode::Down => {
                        self.is_backward_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::D | VirtualKeyCode::Right => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn update_camera(&self, camera: &mut Camera) {
        // ターゲットからカメラの座標を引く(カメラから見たターゲットの向き)
        let forward = camera.target - camera.eye;
        // 向きに対する単位行列
        let forward_norm = forward.normalize();
        // 向きへの距離
        let forward_mag = forward.magnitude();

        if self.is_forward_pressed && forward_mag > self.speed {
            // カメラの位置に向きの単位行列 * 速度分足加える(近づく)
            camera.eye += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            // カメラの位置に向きの単位行列 * 速度分足引く(離れる)
            camera.eye -= forward_norm * self.speed;
        }

        let right = forward_norm.cross(camera.up); // ターゲットへの単位行列と縦軸との外積をとる

        // なぜ再定義が必要？
        let forward = camera.target - camera.eye;
        let forward_mag = forward.magnitude();

        if self.is_right_pressed {
            // ターゲットから、カメラのほうの少し右を見る単位行列を作り、それに元の距離をかける
            camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
        }

        if self.is_up_pressed {
            camera.eye += camera.up * self.speed;
        }
        if self.is_down_pressed {
            camera.eye -= camera.up * self.speed;
        }
    }
}
