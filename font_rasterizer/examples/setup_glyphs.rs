use std::collections::HashSet;

use font_collector::FontCollector;
use font_rasterizer::{
    camera::Camera,
    color_theme::ColorTheme,
    font_buffer::GlyphVertexBuffer,
    instances::GlyphInstances,
    rasterizer_pipeline::Quarity,
    support::{
        run_support, Flags, GlobalStateContext, InputResult, SimpleStateCallback,
        SimpleStateSupport, WindowSize,
    },
};
use instant::Instant;
use winit::event::WindowEvent;

const EMOJI_FONT_DATA: &[u8] = include_bytes!("font/NotoEmoji-Regular.ttf");

pub fn main() {
    std::env::set_var("RUST_LOG", "support_test=debug");
    pollster::block_on(run());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    let mut collector = FontCollector::default();
    collector.add_system_fonts();
    let data = collector.load_font("UD デジタル 教科書体 N-R");

    let font_binaries = vec![
        data.unwrap(),
        collector
            .convert_font(EMOJI_FONT_DATA.to_vec(), None)
            .unwrap(),
    ];

    let callback = SingleCharCallback {
        camera: Camera::basic((800, 600)),
    };
    let support = SimpleStateSupport {
        window_icon: None,
        window_title: "Hello".to_string(),
        window_size: (800, 600),
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
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(&mut self, glyph_vertex_buffer: &mut GlyphVertexBuffer, context: &GlobalStateContext) {
        let start = Instant::now();
        let _char_ranges = [
            'a'..='z',
            'A'..='Z',
            '0'..='9',
            'あ'..='ん',
            'ア'..='ン',
            '亜'..='腺',
            '一'..='龠',
            '㐀'..='䶿',
        ]
        .iter()
        .map(|char_range| char_range.clone().into_iter().collect::<HashSet<char>>())
        .for_each(|c| {
            let start = Instant::now();
            println!("c len: {}", c.len());
            glyph_vertex_buffer
                .append_glyph(&context.device, &context.queue, c)
                .unwrap();
            let end = Instant::now();
            println!("init: {:?}", end - start);
        });
        let end = Instant::now();
        println!("init: {:?}", end - start);
    }

    fn update(
        &mut self,
        _glyph_vertex_buffer: &mut GlyphVertexBuffer,
        _context: &GlobalStateContext,
    ) {
    }

    fn input(
        &mut self,
        _glyph_vertex_buffer: &GlyphVertexBuffer,
        _event: &WindowEvent,
    ) -> InputResult {
        InputResult::Noop
    }

    fn resize(&mut self, _window_size: WindowSize) {}

    fn render(&mut self) -> (&Camera, Vec<&GlyphInstances>) {
        (&self.camera, Vec::new())
    }
}
