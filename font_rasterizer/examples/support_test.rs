use cgmath::Rotation3;
use font_rusterizer::{
    camera::{Camera, CameraController},
    color_theme::ColorTheme::SolarizedDark,
    default_state::SimpleStateCallback,
    instances::{GlyphInstance, GlyphInstances, MotionFlags},
    rasterizer_pipeline::Quarity,
    support::{run_support, Flags, SimpleStateSupport},
};
use log::info;
use winit::event::{ElementState, MouseButton, WindowEvent};

pub fn main() {
    std::env::set_var("RUST_LOG", "support_test=debug");
    pollster::block_on(run());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    let callback = SingleCharCallback::new();
    let support = SimpleStateSupport {
        window_title: "Hello".to_string(),
        window_size: (800, 600),
        callback: Box::new(callback),
        quarity: Quarity::VeryHigh,
        bg_color: SolarizedDark.background().into(),
        flags: Flags::DEFAULT,
    };
    run_support(support).await;
}

struct SingleCharCallback {
    camera: Camera,
    camera_controller: CameraController,
    glyphs: Vec<GlyphInstances>,
    motion: MotionType,
}

#[derive(Debug)]
enum MotionType {
    None,
    WaveX,
    WaveY,
    WaveZ,
    RotateX,
    RotateY,
    RotateZ,
}

impl MotionType {
    fn next(&self) -> Self {
        match self {
            Self::None => Self::WaveX,
            Self::WaveX => Self::WaveY,
            Self::WaveY => Self::WaveZ,
            Self::WaveZ => Self::RotateX,
            Self::RotateX => Self::RotateY,
            Self::RotateY => Self::RotateZ,
            Self::RotateZ => Self::None,
        }
    }
    fn motion_flags(&self) -> MotionFlags {
        match self {
            Self::None => MotionFlags::empty(),
            Self::WaveX => MotionFlags::WAVE_X,
            Self::WaveY => MotionFlags::WAVE_Y,
            Self::WaveZ => MotionFlags::WAVE_Z,
            Self::RotateX => MotionFlags::ROTATE_X,
            Self::RotateY => MotionFlags::ROTATE_Y,
            Self::RotateZ => MotionFlags::ROTATE_Z,
        }
    }
}

impl SingleCharCallback {
    fn new() -> Self {
        Self {
            camera: Camera::new(
                (0.0, 0.0, 1.0).into(),
                (0.0, 0.0, 0.0).into(),
                cgmath::Vector3::unit_y(),
                800 as f32 / 600 as f32,
                // fovy は視野角。ここでは45度を指定
                45.0,
                0.1,
                200.0,
            ),
            camera_controller: CameraController::new(10.0),
            glyphs: Vec::new(),
            motion: MotionType::None,
        }
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(&mut self, device: &wgpu::Device, _queue: &wgpu::Queue) {
        let value = GlyphInstance::new(
            (0.0, 0.0, 0.0).into(),
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
            SolarizedDark.cyan().get_color(),
            self.motion.motion_flags(),
        );
        let mut instance = GlyphInstances::new('あ', Vec::new(), device);
        instance.push(value);
        self.glyphs.push(instance);
    }

    fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.glyphs
            .iter_mut()
            .for_each(|i| i.update_buffer(device, queue));
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.motion = self.motion.next();
                info!("next motion:{:?}", self.motion);
                self.glyphs.iter_mut().for_each(|i| {
                    if i.c == 'あ' {
                        i.clear();
                        i.push(GlyphInstance::new(
                            (0.0, 0.0, 0.0).into(),
                            cgmath::Quaternion::from_axis_angle(
                                cgmath::Vector3::unit_z(),
                                cgmath::Deg(0.0),
                            ),
                            SolarizedDark.cyan().get_color(),
                            self.motion.motion_flags(),
                        ))
                    }
                })
            }
            _ => {}
        }
        false
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.camera_controller
            .update_camera_aspect(&mut self.camera, width, height);
    }

    fn render(&mut self) -> (&Camera, Vec<&GlyphInstances>) {
        (&self.camera, self.glyphs.iter().collect())
    }
}
