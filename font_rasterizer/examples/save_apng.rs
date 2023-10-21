use std::collections::{hash_set, HashSet};

use instant::Duration;

use cgmath::Rotation3;
use font_rasterizer::{
    camera::{Camera, CameraController},
    color_theme::ColorTheme::SolarizedDark,
    font_buffer::GlyphVertexBuffer,
    instances::{GlyphInstance, GlyphInstances},
    motion::{EasingFuncType, MotionDetail, MotionFlags, MotionTarget, MotionType},
    rasterizer_pipeline::Quarity,
    support::{generate_apng, Flags, InputResult, SimpleStateCallback, SimpleStateSupport},
    time::now_millis,
};
use log::{debug, info};
use winit::event::{ElementState, MouseButton, WindowEvent};

const FONT_DATA: &[u8] = include_bytes!("font/HackGenConsole-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] = include_bytes!("font/NotoEmoji-Regular.ttf");

pub fn main() {
    std::env::set_var("RUST_LOG", "debug");
    pollster::block_on(run());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    let font_binaries = vec![FONT_DATA.to_vec(), EMOJI_FONT_DATA.to_vec()];

    let callback = SingleCharCallback::new();
    let support = SimpleStateSupport {
        window_title: "Hello".to_string(),
        window_size: (256, 256),
        callback: Box::new(callback),
        quarity: Quarity::Middle,
        bg_color: SolarizedDark.background().into(),
        flags: Flags::DEFAULT,
        font_binaries,
    };
    generate_apng(support).await;
}

struct SingleCharCallback {
    camera: Camera,
    camera_controller: CameraController,
    glyphs: Vec<GlyphInstances>,
}

impl SingleCharCallback {
    fn new() -> Self {
        Self {
            camera: Camera::basic((256, 256)),
            camera_controller: CameraController::new(10.0),
            glyphs: Vec::new(),
        }
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(
        &mut self,
        _glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) {
        let value = GlyphInstance::new(
            (0.0, 0.0, 0.0).into(),
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
            SolarizedDark.cyan().get_color(),
            MotionFlags::ZERO_MOTION,
            now_millis(),
            2.0,
            Duration::from_millis(1000),
        );
        let mut instance = GlyphInstances::new('あ', Vec::new(), device);
        instance.push(value);
        debug!("init!");
    }

    fn update(
        &mut self,
        _glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.glyphs
            .iter_mut()
            .for_each(|i| i.update_buffer(device, queue));
    }

    fn input(&mut self, event: &WindowEvent) -> InputResult {
        InputResult::Noop
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.camera_controller
            .update_camera_aspect(&mut self.camera, width, height);
    }

    fn render(&mut self) -> (&Camera, Vec<&GlyphInstances>) {
        (&self.camera, self.glyphs.iter().collect())
    }
}
