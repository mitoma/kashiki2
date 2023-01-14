use cgmath::Rotation3;
use font_rusterizer::{
    camera::{Camera, CameraController},
    color_theme::ColorTheme::SolarizedDark,
    default_state::SimpleStateCallback,
    instances::{GlyphInstance, GlyphInstances},
    rasterizer_pipeline::Quarity,
    support::{run_support, Flags, SimpleStateSupport},
};
use winit::event::WindowEvent;

pub fn main() {
    //std::env::set_var("RUST_LOG", "font_rusterizer=debug");
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
        flags: Flags::DEFAULT | Flags::TRANCEPARENT | Flags::NO_TITLEBAR,
    };
    run_support(support).await;
}

struct SingleCharCallback {
    camera: Camera,
    camera_controller: CameraController,
    glyphs: Vec<GlyphInstances>,
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
        }
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(&mut self, device: &wgpu::Device, _queue: &wgpu::Queue) {
        let value = GlyphInstance::new(
            (0.0, 0.0, 0.0).into(),
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
            SolarizedDark.cyan().get_color(),
            2,
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

    fn input(&mut self, _event: &WindowEvent) -> bool {
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
