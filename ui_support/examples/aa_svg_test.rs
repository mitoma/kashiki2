use std::fs;

use apng::{Frame, ParallelEncoder, load_dynamic_image};
use cgmath::One;
use font_collector::{FontCollector, FontRepository};
use web_time::{Duration, SystemTime};

use font_rasterizer::{
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    motion::MotionFlags,
    rasterizer_pipeline::Quarity,
    vector_instances::{InstanceAttributes, VectorInstances},
};
use log::{debug, info};
use ui_support::{
    Flags, InputResult, RenderData, SimpleStateCallback, SimpleStateSupport,
    camera::{Camera, CameraController},
    generate_image_iter,
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
    camera: Camera,
    camera_controller: CameraController,
    vectors: Vec<VectorInstances<String>>,
}

impl SingleCharCallback {
    fn new(window_size: WindowSize) -> Self {
        Self {
            camera: Camera::basic(window_size),
            camera_controller: CameraController::new(10.0),
            vectors: Vec::new(),
        }
    }

    fn register_and_add_svg(
        &mut self,
        context: &StateContext,
        key: &str,
        svg: &str,
        position: (f32, f32, f32),
    ) {
        context.register_svg(key.into(), svg.into());
        let value = InstanceAttributes {
            position: position.into(),
            rotation: cgmath::Quaternion::one(),
            world_scale: [1.0, 1.0],
            instance_scale: [0.5, 0.5],
            color: context.color_theme.text_emphasized().get_color(),
            motion: MotionFlags::ZERO_MOTION,
            ..Default::default()
        };
        let mut instances = VectorInstances::new(key.to_string(), &context.device);
        instances.push(value);
        self.vectors.push(instances);
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(&mut self, context: &StateContext) {
        self.register_and_add_svg(
            context,
            "tri",
            include_str!("../../font_rasterizer/data/test_shapes_tri.svg"),
            (-0.2, -0.2, 0.0),
        );
        self.register_and_add_svg(
            context,
            "tri2",
            include_str!("../../font_rasterizer/data/test_shapes_tri2.svg"),
            (-0.6, -0.2, 0.0),
        );
        self.register_and_add_svg(
            context,
            "bezier",
            include_str!("../../font_rasterizer/data/test_shapes_bezier.svg"),
            (0.2, -0.2, 0.0),
        );
        self.register_and_add_svg(
            context,
            "bezier2",
            include_str!("../../font_rasterizer/data/test_shapes_bezier2.svg"),
            (0.6, -0.2, 0.0),
        );

        self.register_and_add_svg(
            context,
            "square",
            include_str!("../../font_rasterizer/data/test_shapes_square.svg"),
            (-0.6, 0.2, 0.0),
        );
        self.register_and_add_svg(
            context,
            "tri3",
            include_str!("../../font_rasterizer/data/test_shapes_tri3.svg"),
            (-0.2, 0.2, 0.0),
        );

        debug!("init!");
    }

    fn update(&mut self, context: &StateContext) {
        self.vectors
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
            glyph_instances: vec![],
            vector_instances: self.vectors.iter().collect(),
            glyph_instances_for_modal: vec![],
            vector_instances_for_modal: vec![],
        }
    }

    fn shutdown(&mut self) {}
}
