use font_collector::FontCollector;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use font_rasterizer::{
    camera::{Camera, CameraAdjustment, CameraOperation},
    color_theme::ColorTheme,
    font_buffer::GlyphVertexBuffer,
    instances::GlyphInstances,
    layout_engine::{HorizontalWorld, World},
    motion::{MotionDetail, MotionFlags, MotionTarget},
    rasterizer_pipeline::Quarity,
    support::{run_support, Flags, InputResult, SimpleStateCallback, SimpleStateSupport},
    ui::PlaneTextReader,
};
use log::info;
use winit::event::{ElementState, KeyEvent, WindowEvent};

const FONT_DATA: &[u8] =
    include_bytes!("../../../font_rasterizer/examples/font/HackGenConsole-Regular.ttf");
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

    let callback = SingleCharCallback::new();
    let support = SimpleStateSupport {
        window_title: "スライドデモ".to_string(),
        window_size: (800, 600),
        callback: Box::new(callback),
        quarity: Quarity::High,
        bg_color: ColorTheme::SolarizedDark.background().into(),
        flags: Flags::DEFAULT,
        font_binaries,
    };
    run_support(support).await;
}

struct SingleCharCallback {
    world: Box<dyn World>,
    color_theme: ColorTheme,
    look_at: usize,
}

impl SingleCharCallback {
    fn new() -> Self {
        let mut world = Box::new(HorizontalWorld::new(800, 600));
        let slide = include_str!("slide.md");

        let parser = pulldown_cmark::Parser::new(slide);
        let mut text_buffer = String::new();
        parser.for_each(|event| {
            match event {
                pulldown_cmark::Event::Text(text) => {
                    info!("push text: {}", text);
                    text_buffer.push_str(&text);
                }
                pulldown_cmark::Event::SoftBreak => {
                    text_buffer.push('\n');
                }
                pulldown_cmark::Event::End(_) => {
                    info!("text_buffer: {}", text_buffer);
                    let mut model = Box::new(PlaneTextReader::new(text_buffer.clone()));
                    model.update_motion(
                        MotionFlags::builder()
                            .motion_type(font_rasterizer::motion::MotionType::EaseInOut(
                                font_rasterizer::motion::EasingFuncType::Sin,
                                true,
                            ))
                            .motion_detail(MotionDetail::USE_Y_DISTANCE)
                            .motion_target(MotionTarget::MOVE_X_PLUS)
                            .build(),
                    );
                    world.add(model);
                    text_buffer.clear();
                }
                /*
                pulldown_cmark::Event::Start(_) => todo!(),
                pulldown_cmark::Event::End(_) => todo!(),
                pulldown_cmark::Event::Code(_) => todo!(),
                pulldown_cmark::Event::Html(_) => todo!(),
                pulldown_cmark::Event::FootnoteReference(_) => todo!(),
                pulldown_cmark::Event::SoftBreak => todo!(),
                pulldown_cmark::Event::HardBreak => todo!(),
                pulldown_cmark::Event::Rule => todo!(),
                pulldown_cmark::Event::TaskListMarker(_) => todo!(),
                 */
                event => info!("event: {:?}", event),
            }
        });
        info!("world.model_length: {}", world.model_length());

        let look_at = 0;
        world.look_at(look_at, CameraAdjustment::FitBoth);
        world.re_layout();

        Self {
            world,
            color_theme: ColorTheme::SolarizedDark,
            look_at,
        }
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(
        &mut self,
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.update(glyph_vertex_buffer, device, queue);
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.world.change_window_size((width, height));
    }

    fn update(
        &mut self,
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.world.re_layout();
        self.world
            .update(&self.color_theme, glyph_vertex_buffer, device, queue);
    }

    fn input(&mut self, event: &WindowEvent) -> InputResult {
        if let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    state: ElementState::Pressed,
                    logical_key,
                    ..
                },
            ..
        } = event
        {
            info!("key: {:?}", logical_key);
            match logical_key.as_ref() {
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowRight) => {
                    self.look_at += 1;
                    self.look_at %= self.world.model_length();
                    self.world.look_at(self.look_at, CameraAdjustment::FitBoth);
                }
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowLeft) => {
                    self.look_at += self.world.model_length() - 1;
                    self.look_at %= self.world.model_length();
                    self.world.look_at(self.look_at, CameraAdjustment::FitBoth);
                }
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowUp) => {
                    self.world.look_at(self.look_at, CameraAdjustment::FitWidth);
                }
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowDown) => {
                    self.world
                        .look_at(self.look_at, CameraAdjustment::FitHeight);
                }
                winit::keyboard::Key::Character("w") => {
                    self.world.camera_operation(CameraOperation::Up);
                }
                winit::keyboard::Key::Character("s") => {
                    self.world.camera_operation(CameraOperation::Down);
                }
                winit::keyboard::Key::Character("a") => {
                    self.world.camera_operation(CameraOperation::Left);
                }
                winit::keyboard::Key::Character("d") => {
                    self.world.camera_operation(CameraOperation::Right);
                }
                winit::keyboard::Key::Character("z") => {
                    self.world.camera_operation(CameraOperation::Forward);
                }
                winit::keyboard::Key::Character("x") => {
                    self.world.camera_operation(CameraOperation::Backward);
                }
                _ => {}
            }
        }
        InputResult::Noop
    }

    fn render(&mut self) -> (&Camera, Vec<&GlyphInstances>) {
        let instances = self.world.glyph_instances();
        (self.world.camera(), instances)
    }
}
