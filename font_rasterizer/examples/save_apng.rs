use std::{
    collections::HashSet,
    fs::File,
    io::BufWriter,
    sync::{mpsc::SyncSender, Arc, Mutex},
};

use apng::{load_dynamic_image, Encoder, Frame, PNGImage};
use instant::Duration;

use cgmath::Rotation3;
use font_rasterizer::{
    camera::{Camera, CameraController},
    color_theme::ColorTheme::SolarizedDark,
    font_buffer::GlyphVertexBuffer,
    instances::{GlyphInstance, GlyphInstances},
    motion::{EasingFuncType, MotionDetail, MotionFlags, MotionTarget, MotionType},
    rasterizer_pipeline::Quarity,
    support::{generate_image_iter, Flags, InputResult, SimpleStateCallback, SimpleStateSupport},
    time::now_millis,
};
use log::{debug, info};
use winit::event::WindowEvent;

const FONT_DATA: &[u8] = include_bytes!("font/HackGenConsole-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] = include_bytes!("font/NotoEmoji-Regular.ttf");

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
    let font_binaries = vec![FONT_DATA.to_vec(), EMOJI_FONT_DATA.to_vec()];

    let callback = SingleCharCallback::new();
    let support = SimpleStateSupport {
        window_title: "Hello".to_string(),
        window_size: (512, 512),
        callback: Box::new(callback),
        quarity: Quarity::VeryHigh,
        bg_color: SolarizedDark.background().into(),
        flags: Flags::DEFAULT,
        font_binaries,
    };

    info!("start generate images");
    let num_of_frame = 1024;

    info!("start apng encode");

    let image_sender: Box<Mutex<Option<SyncSender<(PNGImage, Option<Frame>)>>>> =
        Box::new(Mutex::new(None));

    let mut image_iter =
        generate_image_iter(support, num_of_frame, Duration::from_millis(20)).await;
    let image_iter = Arc::new(Mutex::new(image_iter));

    let (first_image, frame) = image_iter.lock().unwrap().next().unwrap();

    let dynimage = image::DynamicImage::ImageRgba8(first_image);
    let png_image = load_dynamic_image(dynimage).unwrap();

    let config = apng::create_config_with_num_frames(&png_image, num_of_frame, None).unwrap();
    let file_name = "test-animation";
    let out = BufWriter::new(File::create(format!("{}.png", file_name)).unwrap());
    let frame = Frame {
        delay_num: Some(1),
        delay_den: Some(50),
        ..Default::default()
    };
    
    Encoder::encode_parallel(out, config, Some(frame), move |sender| {
        sender.send((png_image.clone(), None)).unwrap();
        image_iter.lock().unwrap().for_each(|(img, frame)| {
            let dynimage = image::DynamicImage::ImageRgba8(img);
            let png_image = load_dynamic_image(dynimage).unwrap();
            sender.send((png_image, None)).unwrap();
        });
    })
    .unwrap();

    /*

               info!("start frame {}", frame);
               let dynimage = image::DynamicImage::ImageRgba8(image);
               let png_image = load_dynamic_image(dynimage).unwrap();
               if frame == 0 {
                   let config =
                       apng::create_config_with_num_frames(&png_image, num_of_frame, None).unwrap();
                   let file_name = "test-animation";
                   let out = BufWriter::new(File::create(format!("{}.png", file_name)).unwrap());
                   let frame = Frame {
                       delay_num: Some(1),
                       delay_den: Some(50),
                       ..Default::default()
                   };
                   Encoder::encode_parallel(out, config, Some(frame), |sender| {
                       info!("replace sender");
                       image_sender.lock().unwrap().replace(sender);
                       info!("replaced sender");
                   })
                   .unwrap();
               }
               info!("send image");
               image_sender
                   .lock()
                   .unwrap()
                   .as_ref()
                   .unwrap()
                   .send((png_image, None))
                   .unwrap();
               info!("sended!");
           },
       )
       .await;
    */
    info!("finish!");
}

struct SingleCharCallback {
    camera: Camera,
    camera_controller: CameraController,
    glyphs: Vec<GlyphInstances>,
}

impl SingleCharCallback {
    fn new() -> Self {
        Self {
            camera: Camera::basic((256, 256)),
            camera_controller: CameraController::new(10.0),
            glyphs: Vec::new(),
        }
    }
}

impl SimpleStateCallback for SingleCharCallback {
    fn init(
        &mut self,
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let value = GlyphInstance::new(
            (0.0, 0.0, 0.0).into(),
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
            SolarizedDark.cyan().get_color(),
            //MotionFlags::ZERO_MOTION,
            MotionFlags::new(
                MotionType::EaseInOut(EasingFuncType::Quad, true),
                MotionDetail::USE_X_DISTANCE,
                MotionTarget::MOVE_X_PLUS,
            ),
            now_millis(),
            2.0,
            Duration::from_millis(1000),
        );
        let mut instances = GlyphInstances::new('あ', Vec::new(), device);
        instances.push(value);
        self.glyphs.push(instances);
        let mut chars = HashSet::<char>::new();
        chars.insert('あ');
        glyph_vertex_buffer
            .append_glyph(device, queue, chars)
            .unwrap();
        debug!("init!");
    }

    fn update(
        &mut self,
        _glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.glyphs
            .iter_mut()
            .for_each(|i| i.update_buffer(device, queue));
    }

    fn input(&mut self, _event: &WindowEvent) -> InputResult {
        InputResult::Noop
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.camera_controller
            .update_camera_aspect(&mut self.camera, width, height);
    }

    fn render(&mut self) -> (&Camera, Vec<&GlyphInstances>) {
        (&self.camera, self.glyphs.iter().collect())
    }
}
