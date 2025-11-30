use std::fs::File;

use font_collector::{FontCollector, FontRepository};
use image::{Delay, Frame, codecs::gif::GifEncoder};
use web_time::Duration;

use font_rasterizer::{color_theme::ColorTheme, context::WindowSize, rasterizer_pipeline::Quarity};
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

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    let mut font_collector = FontCollector::default();
    font_collector.add_system_fonts();
    let mut font_repository = FontRepository::new(font_collector);

    font_repository.add_fallback_font_from_system("UD デジタル 教科書体 N-R");
    font_repository.add_fallback_font_from_system("Segoe UI Emoji");

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

    let path = std::path::Path::new("test-animation.gif");
    let writer = File::create(path).unwrap();

    let image_iter = generate_image_iter(support, num_of_frame, Duration::from_millis(20))
        .await
        .map(|(image, index)| {
            let frame = Frame::from_parts(
                image,
                0,
                0,
                Delay::from_saturating_duration(Duration::from_millis(20)),
            );

            (frame, index)
        });
    //let (image, _idx) = image_iter.next().unwrap();

    let mut encoder = GifEncoder::new(writer);
    let _ = encoder.set_repeat(image::codecs::gif::Repeat::Infinite);
    for (img_frame, idx) in image_iter {
        info!("send image to encoder. frame: {}", idx);
        let _ = encoder.encode_frame(img_frame);
        info!("sended image to encoder. frame: {}", idx);
    }
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
    fn init(&mut self, context: &UiContext) {
        self.world.add(Box::new(TextEdit::default()));
        context.register_string("エディタの文字をアニメーションGifにほげ".to_string());
        self.world.editor_operation(&EditorOperation::InsertString(
            "エディタの文字をアニメーションGifに".to_string(),
        ));
        self.world
            .model_operation(&ModelOperation::ChangeDirection(None));
        self.world.look_at(0, CameraAdjustment::FitBoth);
        context.register_post_action(Action::new_command("world", "reset-zoom"));
        context.register_post_action(Action::new_command_with_argument(
            "system",
            "change-background-image",
            "kashikishi/asset/image/wallpaper.jpg",
        ));
        self.world.editor_operation(&EditorOperation::InsertEnter);
        self.world
            .editor_operation(&EditorOperation::InsertString("ほげほげ".to_string()));
    }

    fn update(&mut self, context: &UiContext) {
        self.world.update(context);
    }

    fn input(&mut self, _context: &UiContext, _event: &WindowEvent) -> InputResult {
        InputResult::Noop
    }

    fn action(&mut self, context: &UiContext, action: stroke_parser::Action) -> InputResult {
        self.action_processor_store
            .process(&action, context, &mut self.world)
    }

    fn resize(&mut self, window_size: WindowSize) {
        self.world.change_window_size(window_size);
    }

    fn render(&'_ mut self) -> RenderData<'_> {
        RenderData {
            camera: self.world.camera(),
            glyph_instances: self.world.glyph_instances(),
            vector_instances: vec![],
            glyph_instances_for_modal: vec![],
            vector_instances_for_modal: vec![],
        }
    }

    fn shutdown(&mut self) {}
}
