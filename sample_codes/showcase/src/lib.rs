use font_collector::FontCollector;
use instant::Duration;
use stroke_parser::Action;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use cgmath::Rotation3;
use font_rasterizer::{
    camera::{Camera, CameraController},
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    font_buffer::GlyphVertexBuffer,
    instances::{GlyphInstance, GlyphInstances},
    motion::{EasingFuncType, MotionDetail, MotionFlags, MotionTarget, MotionType},
    rasterizer_pipeline::Quarity,
    support::{run_support, Flags, InputResult, SimpleStateCallback, SimpleStateSupport},
    time::now_millis,
};
use log::info;
use winit::event::{ElementState, MouseButton, WindowEvent};

const FONT_DATA: &[u8] =
    include_bytes!("../../../font_rasterizer/examples/font/BIZUDMincho-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] =
    include_bytes!("../../../font_rasterizer/examples/font/NotoEmoji-Regular.ttf");

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
        performance_mode: false,
    };
    run_support(support).await;
}

struct SingleCharCallback {
    camera: Camera,
    camera_controller: CameraController,
    glyphs: Vec<GlyphInstances>,
    motion: MyMotion,
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
        }
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(&mut self, glyph_vertex_buffer: &mut GlyphVertexBuffer, context: &StateContext) {
        let value = GlyphInstance {
            color: context.color_theme.cyan().get_color(),
            motion: self.motion.motion_flags(),
            gain: 2.0,
            duration: Duration::from_millis(1000),
            ..Default::default()
        };
        let mut instance = GlyphInstances::new('あ', &context.device);
        instance.push(value);
        self.glyphs.push(instance);
        let chars = vec!['あ'].into_iter().collect();
        glyph_vertex_buffer
            .append_glyph(&context.device, &context.queue, chars)
            .unwrap();
    }

    fn update(&mut self, _glyph_vertex_buffer: &mut GlyphVertexBuffer, context: &StateContext) {
        self.glyphs
            .iter_mut()
            .for_each(|i| i.update_buffer(&context.device, &context.queue));
    }

    fn input(&mut self, _context: &StateContext, event: &WindowEvent) -> InputResult {
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
                            [1.0, 1.0],
                            [1.0, 1.0],
                            ColorTheme::SolarizedDark.cyan().get_color(),
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

    fn action(&mut self, _context: &StateContext, _action: Action) -> InputResult {
        InputResult::Noop
    }

    fn resize(&mut self, size: WindowSize) {
        self.camera_controller
            .update_camera_aspect(&mut self.camera, size);
    }

    fn render(&mut self) -> (&Camera, Vec<&GlyphInstances>) {
        (&self.camera, self.glyphs.iter().collect())
    }
}
