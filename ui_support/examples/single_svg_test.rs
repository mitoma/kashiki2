use std::fs;

use apng::{Frame, ParallelEncoder, load_dynamic_image};
use font_collector::{FontCollector, FontRepository};
use web_time::{Duration, SystemTime};

use font_rasterizer::{
    color_theme::{ColorTheme, ThemedColor},
    context::WindowSize,
    rasterizer_pipeline::Quarity,
};
use log::{debug, info};
use ui_support::{
    Flags, InputResult, RenderData, SimpleStateCallback, SimpleStateSupport, generate_image_iter,
    layout_engine::{DefaultWorld, World},
    ui::SingleSvg,
    ui_context::UiContext,
};
use winit::event::WindowEvent;

pub fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .init();
    pollster::block_on(run());
}

pub async fn run() {
    let mut font_collector = FontCollector::default();
    font_collector.add_system_fonts();
    let font_repository = FontRepository::new(font_collector);

    let window_size = WindowSize::new(512, 256);
    let callback = SingleCharCallback::new(window_size);
    let support = SimpleStateSupport {
        window_icon: None,
        window_title: "Hello".to_string(),
        window_size,
        callback: Box::new(callback),
        quarity: Quarity::Middle,
        color_theme: ColorTheme::SolarizedBlackback,
        flags: Flags::DEFAULT,
        font_repository,
        performance_mode: false,
        background_image: None,
    };

    info!("start generate images");
    let num_of_frame = 1;

    info!("start apng encode");

    let filename = format!(
        "target/test-svg-aa-{}.png",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    let path = std::path::Path::new(&filename);
    let frame = Frame {
        delay_num: Some(1),
        delay_den: Some(50),
        ..Default::default()
    };

    let mut image_iter = generate_image_iter(support, num_of_frame, Duration::from_millis(20))
        .await
        .map(|(image, index)| {
            let dynimage = image::DynamicImage::ImageRgba8(image);
            let png_image = load_dynamic_image(dynimage).unwrap();
            (png_image, index)
        });
    let (image, _idx) = image_iter.next().unwrap();

    let encoder = ParallelEncoder::new(
        path.to_path_buf(),
        image,
        Some(frame),
        num_of_frame,
        None,
        Some(64),
    )
    .unwrap();
    for (png_image, idx) in image_iter {
        info!("send image to encoder. frame: {}", idx);
        encoder.send(png_image);
        info!("sended image to encoder. frame: {}", idx);
    }
    encoder.finalize();
    fs::copy(filename, "target/test-svg-aa.png").unwrap();
    info!("finish!");
}

struct SingleCharCallback {
    world: DefaultWorld,
}

impl SingleCharCallback {
    fn new(window_size: WindowSize) -> Self {
        let world = DefaultWorld::new(window_size);
        Self { world }
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(&mut self, context: &UiContext) {
        let svg = SingleSvg::new(
            include_str!("../asset/kashikishi-icon-toon-flat.svg").to_string(),
            context,
            ThemedColor::Violet,
        );
        self.world.add(Box::new(svg));
        self.world.re_layout();
        debug!("init!");
    }

    fn update(&mut self, context: &UiContext) {
        self.world.update(context);
    }

    fn input(&mut self, _context: &UiContext, _event: &WindowEvent) -> InputResult {
        InputResult::Noop
    }

    fn action(&mut self, _context: &UiContext, _action: stroke_parser::Action) -> InputResult {
        InputResult::Noop
    }

    fn resize(&mut self, window_size: WindowSize) {
        self.world.change_window_size(window_size);
    }

    fn render(&'_ mut self) -> RenderData<'_> {
        RenderData {
            camera: self.world.camera(),
            glyph_instances: vec![],
            vector_instances: self.world.vector_instances(),
            glyph_instances_for_modal: vec![],
            vector_instances_for_modal: vec![],
        }
    }

    fn shutdown(&mut self) {}
}
