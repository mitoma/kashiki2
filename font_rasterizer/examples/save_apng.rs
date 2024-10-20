use apng::{load_dynamic_image, Frame, ParallelEncoder};
use font_collector::FontCollector;
use instant::Duration;

use cgmath::Rotation3;
use font_rasterizer::{
    camera::{Camera, CameraController},
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    font_buffer::GlyphVertexBuffer,
    instances::{GlyphInstance, GlyphInstances},
    motion::{EasingFuncType, MotionDetail, MotionFlags, MotionTarget, MotionType},
    rasterizer_pipeline::Quarity,
    support::{generate_image_iter, Flags, InputResult, SimpleStateCallback, SimpleStateSupport},
    time::now_millis,
};
use log::{debug, info};
use winit::event::WindowEvent;

//const FONT_DATA: &[u8] = include_bytes!("font/HackGenConsole-Regular.ttf");
//const EMOJI_FONT_DATA: &[u8] = include_bytes!("font/NotoEmoji-Regular.ttf");

pub fn main() {
    std::env::set_var("RUST_LOG", "info");
    env_logger::builder()
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .init();
    //env_logger::init();
    pollster::block_on(run());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    let mut collector = FontCollector::default();
    collector.add_system_fonts();

    let data = collector.load_font("UD デジタル 教科書体 N-R");
    let emoji_data = collector.load_font("Segoe UI Emoji");
    let font_binaries = vec![data.unwrap(), emoji_data.unwrap()];

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
        font_binaries,
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
    fn init(&mut self, _glyph_vertex_buffer: &mut GlyphVertexBuffer, context: &StateContext) {
        let value = GlyphInstance::new(
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
        debug!("init!");
    }

    fn update(&mut self, _glyph_vertex_buffer: &mut GlyphVertexBuffer, context: &StateContext) {
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

    fn render(&mut self) -> (&Camera, Vec<&GlyphInstances>) {
        (&self.camera, self.glyphs.iter().collect())
    }

    fn shutdown(&mut self) {}
}
