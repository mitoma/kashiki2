use font_collector::{FontCollector, FontRepository};
use font_rasterizer::{
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    glyph_instances::GlyphInstances,
    rasterizer_pipeline::Quarity,
};
use instant::Instant;
use ui_support::{
    Flags, InputResult, SimpleStateCallback, SimpleStateSupport, camera::Camera, run_support,
};
use winit::event::WindowEvent;

const EMOJI_FONT_DATA: &[u8] = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");

pub fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .init();
    pollster::block_on(run());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    let mut font_collector = FontCollector::default();
    font_collector.add_system_fonts();
    let mut font_repository = FontRepository::new(font_collector);
    font_repository.set_primary_font("UD デジタル 教科書体 N-R");
    font_repository.add_fallback_font_from_binary(EMOJI_FONT_DATA.to_vec(), None);

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
        font_repository,
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
        [
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
        .map(|char_range| char_range.clone().collect::<String>())
        .for_each(|c| {
            let start = Instant::now();
            println!("c len: {}", c.len());
            context.register_string(c);
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
