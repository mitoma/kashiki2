use apng::{Frame, ParallelEncoder, load_dynamic_image};
use font_collector::{FontCollector, FontRepository};
use instant::Duration;

use font_rasterizer::{
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    rasterizer_pipeline::Quarity,
};
use log::info;
use stroke_parser::Action;
use text_buffer::action::EditorOperation;
use ui_support::{
    Flags, InputResult, RenderData, SimpleStateCallback, SimpleStateSupport,
    action::ActionProcessorStore,
    camera::CameraAdjustment,
    generate_image_iter,
    layout_engine::{DefaultWorld, ModelOperation, World},
    ui::TextEdit,
};
use winit::event::WindowEvent;

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

    font_repository.add_fallback_font_from_system("UD デジタル 教科書体 N-R");

    let window_size = WindowSize::new(512, 512);
    let callback = SingleCharCallback::new(window_size);
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

    info!("start generate images");
    let num_of_frame = 100;

    info!("start apng encode");

    let path = std::path::Path::new("test-animation2.png");
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
    info!("finish!");
}

struct SingleCharCallback {
    world: DefaultWorld,
    action_processor_store: ActionProcessorStore,
}

impl SingleCharCallback {
    fn new(window_size: WindowSize) -> Self {
        let world = DefaultWorld::new(window_size);
        let mut action_processor_store = ActionProcessorStore::default();
        action_processor_store.add_default_system_processors();
        action_processor_store.add_default_world_processors();
        action_processor_store.add_default_edit_processors();
        Self {
            world,
            action_processor_store,
        }
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(&mut self, context: &StateContext) {
        self.world.add(Box::new(TextEdit::default()));
        context.register_string("エディタの文字をアニメーションGifにほげ".to_string());
        self.world.editor_operation(&EditorOperation::InsertString(
            "エディタの文字をアニメーションGifに".to_string(),
        ));
        self.world
            .model_operation(&ModelOperation::ChangeDirection(None));
        self.world.look_at(0, CameraAdjustment::FitBoth);
        context.register_post_action(Action::new_command("world", "reset-zoom"));
        context.register_post_action(stroke_parser::Action::new_command_with_argument(
            "system",
            "change-background-image",
            "kashikishi/asset/image/wallpaper.jpg",
        ));
        self.world.editor_operation(&EditorOperation::InsertEnter);
        self.world
            .editor_operation(&EditorOperation::InsertString("ほげほげ".to_string()));
    }

    fn update(&mut self, context: &StateContext) {
        self.world.update(context);
    }

    fn input(&mut self, _context: &StateContext, _event: &WindowEvent) -> InputResult {
        InputResult::Noop
    }

    fn action(&mut self, context: &StateContext, action: stroke_parser::Action) -> InputResult {
        self.action_processor_store
            .process(&action, context, &mut self.world)
    }

    fn resize(&mut self, window_size: WindowSize) {
        self.world.change_window_size(window_size);
    }

    fn render(&mut self) -> RenderData {
        RenderData {
            camera: self.world.camera(),
            glyph_instances: self.world.glyph_instances(),
            vector_instances: vec![],
        }
    }

    fn shutdown(&mut self) {}
}
