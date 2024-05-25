use font_collector::FontCollector;
use instant::Duration;
use rokid_3dof::RokidMax;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use cgmath::Rotation3;
use font_rasterizer::{
    camera::{Camera, CameraController},
    color_theme::ColorTheme::{self, SolarizedDark},
    context::{StateContext, WindowSize},
    font_buffer::GlyphVertexBuffer,
    instances::{GlyphInstance, GlyphInstances, InstanceKey},
    motion::{EasingFuncType, MotionDetail, MotionFlags, MotionTarget, MotionType},
    rasterizer_pipeline::Quarity,
    support::{run_support, Flags, InputResult, SimpleStateCallback, SimpleStateSupport},
    time::now_millis,
};
use log::info;
use winit::event::{ElementState, MouseButton, WindowEvent};

const FONT_DATA: &[u8] =
    include_bytes!("../../../font_rasterizer/examples/font/HackGenConsole-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] =
    include_bytes!("../../../font_rasterizer/examples/font/NotoEmoji-Regular.ttf");

pub fn main() {
    std::env::set_var("RUST_LOG", "support_test=debug");
    pollster::block_on(run());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    let collector = FontCollector::default();
    let font_binaries = vec![
        collector.convert_font(FONT_DATA.to_vec(), None).unwrap(),
        collector
            .convert_font(EMOJI_FONT_DATA.to_vec(), None)
            .unwrap(),
    ];

    let window_size = WindowSize::new(800, 600);
    let callback = SingleCharCallback::new(window_size);
    let support = SimpleStateSupport {
        window_icon: None,
        window_title: "Hello".to_string(),
        window_size,
        callback: Box::new(callback),
        quarity: Quarity::VeryHigh,
        color_theme: ColorTheme::SolarizedDark,
        flags: Flags::DEFAULT,
        font_binaries,
    };
    run_support(support).await;
}

struct SingleCharCallback {
    camera: Camera,
    camera_controller: CameraController,
    glyphs: Vec<GlyphInstances>,
    motion: MyMotion,
    rokid_max: RokidMax,
}

#[derive(Debug)]
enum MyMotion {
    None,
    WaveX,
    WaveY,
}

impl MyMotion {
    fn next(&self) -> Self {
        match self {
            Self::None => Self::WaveX,
            Self::WaveX => Self::WaveY,
            Self::WaveY => Self::None,
        }
    }
    fn motion_flags(&self) -> MotionFlags {
        match self {
            Self::None => MotionFlags::ZERO_MOTION,
            Self::WaveX => MotionFlags::builder()
                .motion_type(MotionType::EaseOut(EasingFuncType::Sin, false))
                .motion_target(MotionTarget::ROTATE_Z_PLUS)
                .build(),
            Self::WaveY => MotionFlags::builder()
                .motion_type(MotionType::EaseOut(EasingFuncType::Sin, false))
                .motion_detail(MotionDetail::USE_XY_DISTANCE)
                .motion_target(MotionTarget::ROTATE_Z_MINUX)
                .build(),
        }
    }
}

impl SingleCharCallback {
    fn new(window_size: WindowSize) -> Self {
        Self {
            camera: Camera::basic(window_size),
            camera_controller: CameraController::new(10.0),
            glyphs: Vec::new(),
            motion: MyMotion::None,
            rokid_max: RokidMax::new().unwrap(),
        }
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(&mut self, glyph_vertex_buffer: &mut GlyphVertexBuffer, context: &StateContext) {
        let value = GlyphInstance::new(
            (0.0, 0.0, 0.0).into(),
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
            [1.0, 1.0],
            [1.0, 1.0],
            context.color_theme.cyan().get_color(),
            self.motion.motion_flags(),
            now_millis(),
            2.0,
            Duration::from_millis(1000),
        );
        let mut instance = GlyphInstances::new('あ', &context.device);
        instance.push(value);
        self.glyphs.push(instance);
        glyph_vertex_buffer
            .append_glyph(
                &context.device,
                &context.queue,
                ['あ'].iter().cloned().collect(),
            )
            .unwrap();
    }

    fn update(&mut self, _glyph_vertex_buffer: &mut GlyphVertexBuffer, context: &StateContext) {
        self.rokid_max.update().unwrap();
        self.glyphs.iter_mut().for_each(|i| {
            let instance = i.get_mut(&InstanceKey::Monotonic(0)).unwrap();
            instance.rotation = self.rokid_max.quaternion();
            i.update_buffer(&context.device, &context.queue)
        });
    }

    fn input(
        &mut self,
        _glyph_vertex_buffer: &GlyphVertexBuffer,
        _context: &StateContext,
        event: &WindowEvent,
    ) -> InputResult {
        match event {
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                self.rokid_max.reset().unwrap();
                InputResult::InputConsumed
            }
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
                            [1.0, 1.0],
                            [1.0, 1.0],
                            SolarizedDark.cyan().get_color(),
                            self.motion.motion_flags(),
                            now_millis(),
                            2.0,
                            Duration::from_millis(1000),
                        ))
                    }
                });
                InputResult::InputConsumed
            }
            _ => InputResult::Noop,
        }
    }

    fn action(
        &mut self,
        _glyph_vertex_buffer: &GlyphVertexBuffer,
        _context: &StateContext,
        _action: stroke_parser::Action,
    ) -> InputResult {
        InputResult::Noop
    }

    fn resize(&mut self, window_size: WindowSize) {
        self.camera_controller
            .update_camera_aspect(&mut self.camera, window_size);
    }

    fn render(&mut self) -> (&Camera, Vec<&GlyphInstances>) {
        (&self.camera, self.glyphs.iter().collect())
    }
}
