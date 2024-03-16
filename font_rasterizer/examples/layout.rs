use font_collector::FontCollector;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use font_rasterizer::{
    camera::{Camera, CameraAdjustment},
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
        window_icon: None,
        window_title: "Hello".to_string(),
        window_size: (800, 600),
        callback: Box::new(callback),
        quarity: Quarity::High,
        color_theme: SolarizedDark,
        flags: Flags::DEFAULT,
        font_binaries,
    };
    run_support(support).await;
}

struct SingleCharCallback {
    world: Box<dyn World>,
    look_at: usize,
}

impl SingleCharCallback {
    fn new() -> Self {
        let mut world = Box::new(HorizontalWorld::new(800, 600));
        let model = Box::new(PlaneTextReader::new(
            "WGPU による\nFont Rasterize".to_string(),
        ));
        world.add(model);
        let model = Box::new(PlaneTextReader::new(
            "🍖になっちゃった！ほげほげふがふが".to_string(),
        ));
        world.add(model);
        let model = Box::new(PlaneTextReader::new(
            "縦\n書\nき\nを\nと\nて\nも\nや\nり\nま\nす".to_string(),
        ));
        world.add(model);

        let look_at = 0;
        world.look_at(look_at, CameraAdjustment::FitBoth);
        world.re_layout();

        Self { world, look_at }
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(
        &mut self,
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        color_theme: &ColorTheme,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.update(glyph_vertex_buffer, &color_theme, device, queue);
    }

    fn resize(&mut self, _width: u32, _height: u32) {
        self.world.change_window_size((800, 600));
    }

    fn update(
        &mut self,
        mut glyph_vertex_buffer: &mut GlyphVertexBuffer,
        color_theme: &ColorTheme,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.world.re_layout();
        self.world
            .update(color_theme, &mut glyph_vertex_buffer, &device, &queue);
    }

    fn input(
        &mut self,
        _glyph_vertex_buffer: &GlyphVertexBuffer,
        event: &WindowEvent,
    ) -> InputResult {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key,
                        ..
                    },
                ..
            } => {
                info!("key: {:?}", logical_key);
                match logical_key {
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
