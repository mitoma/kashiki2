use apng::{Frame, ParallelEncoder, load_dynamic_image};
use font_collector::{FontCollector, FontRepository};
use instant::Duration;

use cgmath::Rotation3;
use font_rasterizer::{
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    glyph_instances::GlyphInstances,
    motion::{EasingFuncType, MotionDetail, MotionFlags, MotionTarget, MotionType},
    rasterizer_pipeline::Quarity,
    time::now_millis,
    vector_instances::{InstanceAttributes, VectorInstances},
};
use log::{debug, info};
use ui_support::{
    Flags, Flags, InputResult, InputResult, RenderData, SimpleStateCallback, SimpleStateCallback,
    SimpleStateSupport, SimpleStateSupport,
    camera::{Camera, CameraController},
    generate_image_iter, generate_image_iter,
};
use winit::event::WindowEvent;

//const FONT_DATA: &[u8] = include_bytes!("font/HackGenConsole-Regular.ttf");
//const EMOJI_FONT_DATA: &[u8] = include_bytes!("font/NotoEmoji-Regular.ttf");

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
    //let font_binaries = vec![data.unwrap(), emoji_data.unwrap()];

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

    let path = std::path::Path::new("test-animation.png");
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
    camera: Camera,
    camera_controller: CameraController,
    glyphs: Vec<GlyphInstances>,
}

impl SingleCharCallback {
    fn new(window_size: WindowSize) -> Self {
        Self {
            camera: Camera::basic(window_size),
            camera_controller: CameraController::new(10.0),
            glyphs: Vec::new(),
        }
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(&mut self, context: &StateContext) {
        context.register_string("あ".to_string());
        let value = InstanceAttributes::new(
            (0.0, 0.0, 0.0).into(),
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
            [1.0, 1.0],
            [1.0, 1.0],
            context.color_theme.cyan().get_color(),
            //MotionFlags::ZERO_MOTION,
            MotionFlags::builder()
                .motion_type(MotionType::EaseInOut(EasingFuncType::Sin, true))
                .motion_detail(MotionDetail::USE_X_DISTANCE)
                .motion_target(MotionTarget::MOVE_X_PLUS)
                .build(),
            now_millis(),
            2.0,
            Duration::from_millis(1000),
        );
        let mut instances = GlyphInstances::new('あ', &context.device);
        instances.push(value);
        self.glyphs.push(instances);
        context.register_post_action(stroke_parser::Action::new_command(
            "system",
            "change-background-image",
        ));
        debug!("init!");
    }

    fn update(&mut self, context: &StateContext) {
        self.glyphs
            .iter_mut()
            .for_each(|i| i.update_buffer(&context.device, &context.queue));
    }

    fn input(&mut self, _context: &StateContext, _event: &WindowEvent) -> InputResult {
        InputResult::Noop
    }

    fn action(&mut self, _context: &StateContext, _action: stroke_parser::Action) -> InputResult {
        InputResult::Noop
    }

    fn resize(&mut self, window_size: WindowSize) {
        self.camera_controller
            .update_camera_aspect(&mut self.camera, window_size);
    }

    fn render(&mut self) -> RenderData {
        RenderData {
            camera: &self.camera,
            glyph_instances: self.glyphs.iter().collect(),
            vector_instances: vec![],
        }
    }

    fn shutdown(&mut self) {}
}
