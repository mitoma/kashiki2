use font_collector::FontCollector;
use stroke_parser::Action;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use font_rasterizer::{
    camera::{Camera, CameraAdjustment, CameraOperation},
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    instances::GlyphInstances,
    layout_engine::{HorizontalWorld, World},
    motion::{MotionDetail, MotionFlags, MotionTarget},
    rasterizer_pipeline::Quarity,
    ui::PlaneTextReader,
};
use log::info;
use ui_support::{run_support, Flags, InputResult, SimpleStateCallback, SimpleStateSupport};
use winit::event::{ElementState, KeyEvent, WindowEvent};

const FONT_DATA: &[u8] = include_bytes!("../../../fonts/BIZUDMincho-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] = include_bytes!("../../../fonts/NotoEmoji-Regular.ttf");

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
        window_title: "スライドデモ".to_string(),
        window_size,
        callback: Box::new(callback),
        quarity: Quarity::High,
        color_theme: ColorTheme::SolarizedDark,
        flags: Flags::DEFAULT,
        font_binaries,
        performance_mode: false,
    };
    run_support(support).await;
}

struct SingleCharCallback {
    world: Box<dyn World>,
}

impl SingleCharCallback {
    fn new(window_size: WindowSize) -> Self {
        let mut world = Box::new(HorizontalWorld::new(window_size));
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

        world.re_layout();
        world.look_at(0, CameraAdjustment::FitBoth);

        Self { world }
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(&mut self, context: &StateContext) {
        self.update(context);
    }

    fn resize(&mut self, window_size: WindowSize) {
        self.world.change_window_size(window_size);
    }

    fn update(&mut self, context: &StateContext) {
        self.world.re_layout();
        self.world.update(context);
    }

    fn input(&mut self, _context: &StateContext, event: &WindowEvent) -> InputResult {
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
                    self.world.look_next(CameraAdjustment::FitBoth);
                }
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowLeft) => {
                    self.world.look_prev(CameraAdjustment::FitBoth);
                }
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowUp) => {
                    self.world.look_current(CameraAdjustment::FitWidth);
                }
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowDown) => {
                    self.world.look_current(CameraAdjustment::FitHeight);
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

    fn action(&mut self, _context: &StateContext, _action: Action) -> InputResult {
        InputResult::Noop
    }

    fn render(&mut self) -> (&Camera, Vec<&GlyphInstances>) {
        let instances = self.world.glyph_instances();
        (self.world.camera(), instances)
    }

    fn shutdown(&mut self) {}
}
