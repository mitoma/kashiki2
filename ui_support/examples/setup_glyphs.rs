use font_collector::FontCollector;
use font_rasterizer::{
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    instances::GlyphInstances,
    rasterizer_pipeline::Quarity,
};
use instant::Instant;
use ui_support::{camera::Camera, run_support, Flags, InputResult, SimpleStateCallback, SimpleStateSupport};
use winit::event::WindowEvent;

const EMOJI_FONT_DATA: &[u8] = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");

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

    let window_size = WindowSize::new(800, 600);
    let callback = SingleCharCallback {
        camera: Camera::basic(window_size),
    };
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
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(&mut self, context: &StateContext) {
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
        .map(|char_range| char_range.clone().into_iter().collect::<String>())
        .for_each(|c| {
            let start = Instant::now();
            println!("c len: {}", c.len());
            context.ui_string_sender.send(c).unwrap();
            let end = Instant::now();
            println!("init: {:?}", end - start);
        });
        let end = Instant::now();
        println!("init: {:?}", end - start);
    }

    fn update(&mut self, _context: &StateContext) {}

    fn input(&mut self, _context: &StateContext, _event: &WindowEvent) -> InputResult {
        InputResult::Noop
    }

    fn action(&mut self, _context: &StateContext, _action: stroke_parser::Action) -> InputResult {
        InputResult::Noop
    }

    fn resize(&mut self, _window_size: WindowSize) {}

    fn render(&mut self) -> (&Camera, Vec<&GlyphInstances>) {
        (&self.camera, Vec::new())
    }

    fn shutdown(&mut self) {}
}
