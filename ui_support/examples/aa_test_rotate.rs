use std::fs;

use apng::{Frame, ParallelEncoder, load_dynamic_image};
use clap::Parser;
use font_collector::{FontCollector, FontRepository};
use glam::Quat;
use web_time::{Duration, SystemTime};

use font_rasterizer::{
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    glyph_instances::GlyphInstances,
    motion::{EasingFuncType, MotionDetail, MotionFlags, MotionTarget, MotionType},
    rasterizer_pipeline::Quarity,
    vector_instances::InstanceAttributes,
};
use log::{debug, info};
use ui_support::{
    Flags, InputResult, RenderData, SimpleStateCallback, SimpleStateSupport,
    camera::{Camera, CameraController},
    generate_image_iter,
};
use winit::event::WindowEvent;

const FONT_DATA: &[u8] = include_bytes!("../../fonts/BIZUDMincho-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum QuarityArg {
    VeryHigh,
    High,
    Middle,
    Low,
    VeryLow,
}

impl QuarityArg {
    pub fn to_rasterizer_pipeline(&self) -> Quarity {
        match self {
            QuarityArg::VeryHigh => Quarity::VeryHigh,
            QuarityArg::High => Quarity::High,
            QuarityArg::Middle => Quarity::Middle,
            QuarityArg::Low => Quarity::Low,
            QuarityArg::VeryLow => Quarity::VeryLow,
        }
    }
}

#[derive(Parser, Debug, Clone)]
pub struct Args {
    /// use high performance mode
    #[arg(short, long, default_value = "あ")]
    pub char_of_test: char,

    #[arg(short, long, default_value = "middle")]
    pub quarity: QuarityArg,
}

pub fn main() {
    let args = Args::parse();
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .init();
    pollster::block_on(run(args));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run(args: Args) {
    let mut font_collector = FontCollector::default();
    font_collector.add_system_fonts();
    let mut font_repository = FontRepository::new(font_collector);
    font_repository.add_fallback_font_from_system("UD デジタル 教科書体 N");
    font_repository.add_fallback_font_from_system("UD デジタル 教科書体 N-R");
    font_repository.add_fallback_font_from_system("Noto Sans JP");
    font_repository.add_fallback_font_from_binary(FONT_DATA.to_vec(), None);
    font_repository.add_fallback_font_from_binary(EMOJI_FONT_DATA.to_vec(), None);

    let window_size = WindowSize::new(512, 512);
    let callback = SingleCharCallback::new(window_size, args.char_of_test);
    let support = SimpleStateSupport {
        window_icon: None,
        window_title: "Hello".to_string(),
        window_size,
        callback: Box::new(callback),
        quarity: args.quarity.to_rasterizer_pipeline(),
        color_theme: ColorTheme::SolarizedBlackback,
        flags: Flags::DEFAULT,
        font_repository,
        performance_mode: false,
    };

    info!("start generate images");
    let num_of_frame = 256;

    info!("start apng encode");

    let filename = format!(
        "target/test-font-aa-{}.png",
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
    fs::copy(filename, "target/test-font-aa.png").unwrap();
    info!("finish!");
}

struct SingleCharCallback {
    camera: Camera,
    camera_controller: CameraController,
    glyphs: Vec<GlyphInstances>,
    target_char: char,
}

impl SingleCharCallback {
    fn new(window_size: WindowSize, target_char: char) -> Self {
        Self {
            camera: Camera::basic(window_size),
            camera_controller: CameraController::new(10.0),
            glyphs: Vec::new(),
            target_char,
        }
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(&mut self, context: &StateContext) {
        let value = InstanceAttributes {
            position: (0.0, 0.0, 0.0).into(),
            rotation: Quat::IDENTITY,
            world_scale: [1.0, 1.0],
            instance_scale: [0.5, 0.5],
            color: context.color_theme.text_emphasized().get_color(),
            //motion: MotionFlags::random_motion(),
            motion: MotionFlags::builder()
                .motion_detail(MotionDetail::TURN_BACK)
                .motion_type(MotionType::EaseInOut(EasingFuncType::Liner, true))
                .motion_target(MotionTarget::ROTATE_Y_PLUS)
                .build(),

            gain: 1.0,
            duration: Duration::from_secs(100),
            ..Default::default()
        };
        let mut instance = GlyphInstances::new(self.target_char, &context.device);
        instance.push(value);
        self.glyphs.push(instance);
        let chars = vec![self.target_char].into_iter().collect();
        context.register_string(chars);
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

    fn render(&'_ mut self) -> RenderData<'_> {
        RenderData {
            camera: &self.camera,
            glyph_instances: self.glyphs.iter().collect(),
            vector_instances: vec![],
            glyph_instances_for_modal: vec![],
            vector_instances_for_modal: vec![],
        }
    }

    fn shutdown(&mut self) {}
}
