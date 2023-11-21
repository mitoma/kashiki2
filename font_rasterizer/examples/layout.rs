use font_collector::FontCollector;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use font_rasterizer::{
    camera::Camera,
    color_theme::ColorTheme::{self, SolarizedDark},
    font_buffer::GlyphVertexBuffer,
    instances::GlyphInstances,
    layout_engine::{HorizontalWorld, World},
    rasterizer_pipeline::Quarity,
    support::{run_support, Flags, InputResult, SimpleStateCallback, SimpleStateSupport},
    ui::PlaneTextReader,
};
use log::info;
use winit::event::{ElementState, KeyEvent, WindowEvent};

const FONT_DATA: &[u8] = include_bytes!("font/HackGenConsole-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] = include_bytes!("font/NotoEmoji-Regular.ttf");

pub fn main() {
    std::env::set_var("RUST_LOG", "info");
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

    let callback = SingleCharCallback::new();
    let support = SimpleStateSupport {
        window_title: "Hello".to_string(),
        window_size: (800, 600),
        callback: Box::new(callback),
        quarity: Quarity::High,
        bg_color: SolarizedDark.background().into(),
        flags: Flags::DEFAULT,
        font_binaries,
    };
    run_support(support).await;
}

struct SingleCharCallback {
    world: Box<dyn World>,
    color_theme: ColorTheme,
}

impl SingleCharCallback {
    fn new() -> Self {
        let mut world = Box::new(HorizontalWorld::new(800, 600));
        let model = Box::new(PlaneTextReader::new("🐖ぶたちゃんが".to_string()));
        world.add(model);
        let model = Box::new(PlaneTextReader::new("🍖になっちゃった！".to_string()));
        world.add(model);

        Self {
            world,
            color_theme: ColorTheme::SolarizedDark,
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
        //
    }

    fn resize(&mut self, _width: u32, _height: u32) {
        self.world.change_window_size((800, 600));
    }

    fn update(
        &mut self,
        mut glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.world.update(&mut glyph_vertex_buffer, &device, &queue);
    }

    fn input(&mut self, event: &WindowEvent) -> InputResult {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key,
                        text,
                        ..
                    },
                ..
            } => {
                info!("key: {:?}", logical_key);
                match logical_key {
                    winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowLeft) => {
                        self.world.look_at(0);
                    }
                    winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowRight) => {
                        self.world.look_at(1);
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        InputResult::Noop
    }

    fn render(&mut self) -> (&Camera, Vec<&GlyphInstances>) {
        let instances = self.world.glyph_instances();
        info!("instances: {:?}", instances.len());
        (self.world.camera(), instances)
    }
}
