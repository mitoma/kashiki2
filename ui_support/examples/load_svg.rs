use std::fs::File;

use cgmath::One;
use font_collector::FontRepository;
use image::{Delay, Frame, codecs::gif::GifEncoder};
use instant::Duration;

use font_rasterizer::{
    color_theme::ColorTheme,
    context::{StateContext, WindowSize},
    motion::{EasingFuncType, MotionDetail, MotionFlags, MotionTarget, MotionType},
    rasterizer_pipeline::Quarity,
    time::now_millis,
    vector_instances::{InstanceAttributes, VectorInstances},
};
use log::info;
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
    let font_repository = FontRepository::default();

    let window_size = WindowSize::new(256, 256);
    let callback = SingleCharCallback::new(window_size);
    let support = SimpleStateSupport {
        window_icon: None,
        window_title: "Hello".to_string(),
        window_size,
        callback: Box::new(callback),
        quarity: Quarity::Fixed(1024, 1024),
        color_theme: ColorTheme::SolarizedLight,
        flags: Flags::DEFAULT,
        font_repository,
        performance_mode: false,
    };

    info!("start generate images");
    let num_of_frame = 1;

    info!("start apng encode");

    let path = std::path::Path::new("test-svg.gif");
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
    camera: Camera,
    camera_controller: CameraController,
    vector_instances: Vec<VectorInstances<String>>,
}

impl SingleCharCallback {
    fn new(window_size: WindowSize) -> Self {
        Self {
            camera: Camera::basic(window_size),
            camera_controller: CameraController::new(10.0),
            vector_instances: Vec::new(),
        }
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(&mut self, context: &StateContext) {
        context.register_svg(
            "rice".to_string(),
            //include_str!("../../font_rasterizer/data/sample.svg").to_string(),
            include_str!("../../font_rasterizer/data/rice.svg").to_string(),
        );

        let value = InstanceAttributes::new(
            (0.0, 0.0, 0.0).into(),
            cgmath::Quaternion::one(),
            [1.0, 1.0],
            [0.5, 0.5],
            context.color_theme.orange().get_color(),
            //MotionFlags::ZERO_MOTION,
            MotionFlags::builder()
                .motion_type(MotionType::EaseInOut(EasingFuncType::Quad, true))
                .motion_detail(MotionDetail::USE_X_DISTANCE)
                .motion_target(MotionTarget::MOVE_Y_PLUS)
                .build(),
            now_millis(),
            0.5,
            Duration::from_millis(250),
        );
        let mut instances = VectorInstances::new("rice".to_string(), &context.device);
        instances.push(value);
        self.vector_instances.push(instances);
    }

    fn update(&mut self, context: &StateContext) {
        self.vector_instances
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
            glyph_instances: vec![],
            vector_instances: self.vector_instances.iter().collect(),
        }
    }

    fn shutdown(&mut self) {}
}
