mod acrion_record_converter;
mod args;
mod callback;

use std::{
    io::{Cursor, Write},
    num::NonZeroU32,
    sync::{Arc, Mutex},
    time::Duration,
};

use apng::{Config, Encoder, load_dynamic_image};
#[cfg(target_arch = "wasm32")]
use clap::ValueEnum;
use font_collector::FontRepository;
use font_rasterizer::{color_theme::ColorTheme, context::WindowSize, rasterizer_pipeline::Quarity};
#[cfg(target_arch = "wasm32")]
use js_sys::Uint8Array;
use log::info;
#[cfg(not(target_arch = "wasm32"))]
use rav1e::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use shiguredo_mp4::{
    TrackKind, Uint,
    boxes::{Av01Box, Av1cBox, SampleEntry, VisualSampleEntryFields},
    mux::{Mp4FileMuxer, Mp4FileMuxerOptions, Sample, estimate_maximum_moov_box_size},
};
use ui_support::{Flags, SimpleStateSupport, generate_images, ui_context::CharEasingsPreset};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
pub use crate::args::OutputFormatArg;
#[cfg(target_arch = "wasm32")]
use crate::args::WindowSizeArg;
pub use crate::args::{Args, CharEasingsPresetArg, ColorThemeArg};
use crate::{acrion_record_converter::ActionRecordConverter, callback::Callback};

const FONT_DATA: &[u8] = include_bytes!("../../../fonts/BIZUDMincho-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] = include_bytes!("../../../fonts/NotoEmoji-Regular.ttf");

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Default)]
pub enum OutputFormat {
    #[default]
    Apng,
    Mp4,
}

#[cfg(not(target_arch = "wasm32"))]
impl OutputFormat {
    fn output_path(self) -> &'static str {
        match self {
            Self::Apng => "target/output.png",
            Self::Mp4 => "target/output.mp4",
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<OutputFormatArg> for OutputFormat {
    fn from(value: OutputFormatArg) -> Self {
        match value {
            OutputFormatArg::Apng => Self::Apng,
            OutputFormatArg::Mp4 => Self::Mp4,
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
#[cfg(target_arch = "wasm32")]
pub async fn start() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Debug).expect("Could't initialize logger");
    info!("WASM started");
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[cfg(target_arch = "wasm32")]
pub async fn run_wasm(
    target_string: &str,
    window_size: &str,
    color_theme: &str,
    easing_preset: &str,
    fps: &str,
    transparent_bg: bool,
    font_binary: Option<Uint8Array>,
    background_image_binary: Option<Uint8Array>,
) -> Vec<u8> {
    let window_size = WindowSizeArg::from_str(window_size, true)
        .unwrap_or_default()
        .into();
    let color_theme = ColorThemeArg::from_str(color_theme, true)
        .unwrap_or_default()
        .into();
    let easing_preset = CharEasingsPresetArg::from_str(easing_preset, true)
        .unwrap_or_default()
        .into();
    let fps_num: u32 = fps.parse().unwrap_or(24);

    // Convert Uint8Array to Vec<u8> if provided
    let font_binary_vec = font_binary.map(|arr| arr.to_vec());
    let background_image_binary_vec = background_image_binary.map(|arr| arr.to_vec());

    run(
        target_string,
        window_size,
        color_theme,
        easing_preset,
        fps_num,
        transparent_bg,
        font_binary_vec,
        background_image_binary_vec,
        Some(Box::new(|idx, total| {
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("progress")
                .unwrap()
                .set_inner_html(&format!(
                    "Progress: {}/{} ({:.2}%)",
                    idx,
                    total,
                    (idx as f64 / total as f64) * 100.0
                ));
        })),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn run_native(
    target_string: &str,
    window_size: WindowSize,
    color_theme: ColorTheme,
    easing_preset: CharEasingsPreset,
    fps: u32,
    transparent_bg: bool,
    font_binary: Option<Vec<u8>>,
    background_image_binary: Option<Vec<u8>>,
    output_format: OutputFormat,
) {
    let result = match output_format {
        OutputFormat::Apng => {
            run(
                target_string,
                window_size,
                color_theme,
                easing_preset,
                fps,
                transparent_bg,
                font_binary,
                background_image_binary,
                None,
            )
            .await
        }
        OutputFormat::Mp4 => {
            run_mp4(
                target_string,
                window_size,
                color_theme,
                easing_preset,
                fps,
                transparent_bg,
                font_binary,
                background_image_binary,
                None,
            )
            .await
        }
    };
    let mut file = std::fs::File::create(output_format.output_path()).unwrap();
    file.write_all(&result).unwrap();
}

#[allow(clippy::too_many_arguments)]
fn build_support(
    target_string: &str,
    window_size: WindowSize,
    color_theme: ColorTheme,
    easing_preset: CharEasingsPreset,
    transparent_bg: bool,
    font_binary: Option<Vec<u8>>,
    background_image_binary: Option<Vec<u8>>,
) -> (SimpleStateSupport, u32) {
    let mut repository = ActionRecordConverter::new();
    repository.set_direction_vertical();
    repository.append(target_string);

    let sec = repository.all_time_frames().as_secs() as u32 + 2;

    let mut font_repository = FontRepository::default();
    if let Some(font_data) = font_binary {
        font_repository.add_fallback_font_from_binary(font_data, None);
    }
    font_repository.add_fallback_font_from_binary(FONT_DATA.to_vec(), None);
    font_repository.add_fallback_font_from_binary(EMOJI_FONT_DATA.to_vec(), None);
    log::info!("font_repository initialized.");

    log::info!("loading background image...");
    let background_image = background_image_binary.and_then(|data| {
        image::load_from_memory(&data)
            .map_err(|e| {
                log::error!("Failed to load background image: {}", e);
                e
            })
            .ok()
    });
    log::info!("image is some?: {}", background_image.is_some());

    let callback = Callback::new(window_size, Box::new(repository), easing_preset);
    log::info!("callback new.");
    let support = SimpleStateSupport {
        window_icon: None,
        window_title: "Hello".to_string(),
        window_size,
        callback: Box::new(callback),
        quarity: Quarity::VeryHigh,
        color_theme,
        flags: Flags::DEFAULT
            | if transparent_bg {
                Flags::TRANCEPARENT
            } else {
                Flags::empty()
            },
        font_repository,
        performance_mode: false,
        background_image,
        shader_art: None,
    };
    log::info!("support initialized.");

    (support, sec)
}

#[allow(clippy::too_many_arguments)]
pub async fn run(
    target_string: &str,
    window_size: WindowSize,
    color_theme: ColorTheme,
    easing_preset: CharEasingsPreset,
    fps: u32,
    transparent_bg: bool,
    font_binary: Option<Vec<u8>>,
    background_image_binary: Option<Vec<u8>>,
    per_frame_callback: Option<Box<dyn Fn(u32, u32) + Send + 'static>>,
) -> Vec<u8> {
    let (support, sec) = build_support(
        target_string,
        window_size,
        color_theme,
        easing_preset,
        transparent_bg,
        font_binary,
        background_image_binary,
    );

    // EncoderをスレッドセーフにするためにArc<Mutex<>>でラップする
    type EncoderWrapper = Arc<Mutex<Option<Encoder<Cursor<Vec<u8>>>>>>;
    let encoder: EncoderWrapper = Arc::new(Mutex::new(None));
    let encoder_clone = encoder.clone();
    let frame = apng::Frame {
        delay_num: Some(1),
        delay_den: Some(fps as u16),
        ..Default::default()
    };
    log::info!("frame initialized.");

    generate_images(
        support,
        fps * sec,
        Duration::from_millis(1000 / fps as u64),
        move |image, idx| {
            let dynimage = image::DynamicImage::ImageRgba8(image);
            let png_image = load_dynamic_image(dynimage).unwrap();

            // ロックを1回だけ取得し、このスコープで初期化と書き込みを行う
            let mut encoder_wrapper = encoder_clone.lock().unwrap();
            if encoder_wrapper.is_none() {
                let config = Config {
                    width: window_size.width,
                    height: window_size.height,
                    num_frames: fps * sec,
                    num_plays: 1,
                    color: png_image.color_type,
                    depth: png_image.bit_depth,
                    filter: png::Filter::NoFilter,
                };
                let enc = Encoder::new(Cursor::new(Vec::new()), config).unwrap();
                encoder_wrapper.replace(enc);
            }

            if let Some(enc) = encoder_wrapper.as_mut() {
                info!("encoding... frame: {}/{}", idx + 1, fps * sec);
                if let Some(per_frame_callback) = &per_frame_callback {
                    per_frame_callback(idx + 1, fps * sec);
                }
                enc.write_frame(&png_image, frame.clone()).unwrap();
            }
        },
    )
    .await;

    log::info!("finish!");
    let encoder_lock = encoder.lock().unwrap().take();
    let Some(mut encoder) = encoder_lock else {
        panic!("Encoder was not initialized.");
    };
    encoder.finish_encode().unwrap();
    encoder.get_writer().into_inner()
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
pub async fn run_mp4(
    target_string: &str,
    window_size: WindowSize,
    color_theme: ColorTheme,
    easing_preset: CharEasingsPreset,
    fps: u32,
    transparent_bg: bool,
    font_binary: Option<Vec<u8>>,
    background_image_binary: Option<Vec<u8>>,
    per_frame_callback: Option<Box<dyn Fn(u32, u32) + Send + 'static>>,
) -> Vec<u8> {
    let (support, sec) = build_support(
        target_string,
        window_size,
        color_theme,
        easing_preset,
        transparent_bg,
        font_binary,
        background_image_binary,
    );
    let total_frames = fps * sec;
    let encoder = Arc::new(Mutex::new(Mp4VideoEncoder::new(
        window_size,
        fps,
        total_frames,
    )));
    let encoder_clone = encoder.clone();

    generate_images(
        support,
        total_frames,
        Duration::from_millis(1000 / fps as u64),
        move |image, idx| {
            info!("encoding mp4... frame: {}/{}", idx + 1, total_frames);
            if let Some(per_frame_callback) = &per_frame_callback {
                per_frame_callback(idx + 1, total_frames);
            }
            encoder_clone.lock().unwrap().push_frame(&image);
        },
    )
    .await;

    encoder.lock().unwrap().finish()
}

#[cfg(not(target_arch = "wasm32"))]
struct Mp4VideoEncoder {
    context: rav1e::Context<u8>,
    muxer: Mp4FileMuxer,
    bytes: Vec<u8>,
    sample_entry: Option<SampleEntry>,
    timescale: NonZeroU32,
}

#[cfg(not(target_arch = "wasm32"))]
impl Mp4VideoEncoder {
    fn new(window_size: WindowSize, fps: u32, total_frames: u32) -> Self {
        let mut encoder_config = EncoderConfig::with_speed_preset(10);
        encoder_config.width = window_size.width as usize;
        encoder_config.height = window_size.height as usize;
        encoder_config.time_base = Rational {
            num: 1,
            den: fps as u64,
        };
        encoder_config.chroma_sampling = ChromaSampling::Cs444;
        encoder_config.pixel_range = PixelRange::Full;
        encoder_config.color_description = Some(ColorDescription {
            color_primaries: ColorPrimaries::BT709,
            transfer_characteristics: TransferCharacteristics::SRGB,
            matrix_coefficients: MatrixCoefficients::BT709,
        });
        encoder_config.low_latency = true;
        encoder_config.quantizer = 80;
        encoder_config.min_key_frame_interval = fps as u64;
        encoder_config.max_key_frame_interval = fps as u64 * 2;

        let config = rav1e::Config::new()
            .with_encoder_config(encoder_config)
            .with_threads(0);
        let context = config.new_context().unwrap();

        let muxer = Mp4FileMuxer::with_options(Mp4FileMuxerOptions {
            reserved_moov_box_size: estimate_maximum_moov_box_size(&[total_frames as usize]),
            ..Default::default()
        })
        .unwrap();
        let bytes = muxer.initial_boxes_bytes().to_vec();
        let sample_entry = Some(av1_sample_entry(window_size));

        Self {
            context,
            muxer,
            bytes,
            sample_entry,
            timescale: NonZeroU32::new(fps).unwrap(),
        }
    }

    fn push_frame(&mut self, image: &image::RgbaImage) {
        let mut frame = self.context.new_frame();
        let (y_plane, u_plane, v_plane) = rgba_to_yuv444(image);

        frame.planes[0].copy_from_raw_u8(&y_plane, image.width() as usize, 1);
        frame.planes[1].copy_from_raw_u8(&u_plane, image.width() as usize, 1);
        frame.planes[2].copy_from_raw_u8(&v_plane, image.width() as usize, 1);
        frame.planes[0].pad(image.width() as usize, image.height() as usize);
        frame.planes[1].pad(image.width() as usize, image.height() as usize);
        frame.planes[2].pad(image.width() as usize, image.height() as usize);

        self.context.send_frame(frame).unwrap();
        self.drain_packets(false);
    }

    fn finish(&mut self) -> Vec<u8> {
        self.context.flush();
        self.drain_packets(true);
        let finalized = self.muxer.finalize().unwrap();
        for (offset, bytes) in finalized.offset_and_bytes_pairs() {
            let offset = offset as usize;
            let end = offset + bytes.len();
            if self.bytes.len() < end {
                self.bytes.resize(end, 0);
            }
            self.bytes[offset..end].copy_from_slice(bytes);
        }
        std::mem::take(&mut self.bytes)
    }

    fn drain_packets(&mut self, finishing: bool) {
        loop {
            match self.context.receive_packet() {
                Ok(packet) => self.append_packet(packet),
                Err(EncoderStatus::Encoded) => {}
                Err(EncoderStatus::NeedMoreData) if !finishing => break,
                Err(EncoderStatus::LimitReached) if finishing => break,
                Err(EncoderStatus::NeedMoreData | EncoderStatus::LimitReached) => break,
                Err(err) => panic!("rav1e encoding failed: {err}"),
            }
        }
    }

    fn append_packet(&mut self, packet: Packet<u8>) {
        let data_offset = self.bytes.len() as u64;
        self.bytes.extend_from_slice(&packet.data);
        self.muxer
            .append_sample(&Sample {
                track_kind: TrackKind::Video,
                sample_entry: self.sample_entry.take(),
                keyframe: packet.frame_type == FrameType::KEY,
                timescale: self.timescale,
                duration: 1,
                composition_time_offset: None,
                data_offset,
                data_size: packet.data.len(),
            })
            .unwrap();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn av1_sample_entry(window_size: WindowSize) -> SampleEntry {
    SampleEntry::Av01(Av01Box {
        visual: VisualSampleEntryFields {
            data_reference_index: VisualSampleEntryFields::DEFAULT_DATA_REFERENCE_INDEX,
            width: window_size.width as u16,
            height: window_size.height as u16,
            horizresolution: VisualSampleEntryFields::DEFAULT_HORIZRESOLUTION,
            vertresolution: VisualSampleEntryFields::DEFAULT_VERTRESOLUTION,
            frame_count: VisualSampleEntryFields::DEFAULT_FRAME_COUNT,
            compressorname: VisualSampleEntryFields::NULL_COMPRESSORNAME,
            depth: VisualSampleEntryFields::DEFAULT_DEPTH,
        },
        av1c_box: Av1cBox {
            seq_profile: Uint::new(1),
            seq_level_idx_0: Uint::new(31),
            seq_tier_0: Uint::new(0),
            high_bitdepth: Uint::new(0),
            twelve_bit: Uint::new(0),
            monochrome: Uint::new(0),
            chroma_subsampling_x: Uint::new(0),
            chroma_subsampling_y: Uint::new(0),
            chroma_sample_position: Uint::new(0),
            initial_presentation_delay_minus_one: None,
            config_obus: Vec::new(),
        },
        unknown_boxes: Vec::new(),
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn rgba_to_yuv444(image: &image::RgbaImage) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let width = image.width() as usize;
    let height = image.height() as usize;
    let mut y_plane = vec![0; width * height];
    let mut u_plane = vec![0; width * height];
    let mut v_plane = vec![0; width * height];

    for y in 0..height {
        for x in 0..width {
            let [red, green, blue] = composite_pixel(image.get_pixel(x as u32, y as u32).0);
            let index = y * width + x;
            y_plane[index] = rgb_to_y(red, green, blue);
            u_plane[index] = rgb_to_u(red, green, blue);
            v_plane[index] = rgb_to_v(red, green, blue);
        }
    }

    (y_plane, u_plane, v_plane)
}

#[cfg(not(target_arch = "wasm32"))]
fn composite_pixel([red, green, blue, alpha]: [u8; 4]) -> [u8; 3] {
    [
        premultiply_over_black(red, alpha),
        premultiply_over_black(green, alpha),
        premultiply_over_black(blue, alpha),
    ]
}

#[cfg(not(target_arch = "wasm32"))]
fn premultiply_over_black(color: u8, alpha: u8) -> u8 {
    ((color as u16 * alpha as u16 + 127) / 255) as u8
}

#[cfg(not(target_arch = "wasm32"))]
fn rgb_to_y(red: u8, green: u8, blue: u8) -> u8 {
    yuv_component(54 * red as i32 + 183 * green as i32 + 18 * blue as i32, 0)
}

#[cfg(not(target_arch = "wasm32"))]
fn rgb_to_u(red: u8, green: u8, blue: u8) -> u8 {
    yuv_component(
        -29 * red as i32 - 99 * green as i32 + 128 * blue as i32,
        128,
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn rgb_to_v(red: u8, green: u8, blue: u8) -> u8 {
    yuv_component(
        128 * red as i32 - 116 * green as i32 - 12 * blue as i32,
        128,
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn yuv_component(value: i32, offset: i32) -> u8 {
    ((value + 128) >> 8).saturating_add(offset).clamp(0, 255) as u8
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn mp4_encoder_outputs_mp4_boxes() {
        let window_size = WindowSize::new(16, 16);
        let mut encoder = Mp4VideoEncoder::new(window_size, 24, 2);
        let first = image::RgbaImage::from_pixel(16, 16, image::Rgba([255, 0, 0, 255]));
        let second = image::RgbaImage::from_pixel(16, 16, image::Rgba([0, 0, 255, 255]));

        encoder.push_frame(&first);
        encoder.push_frame(&second);
        let mp4 = encoder.finish();

        assert!(mp4.windows(4).any(|bytes| bytes == b"ftyp"));
        assert!(mp4.windows(4).any(|bytes| bytes == b"mdat"));
        assert!(mp4.windows(4).any(|bytes| bytes == b"moov"));
        assert!(mp4.windows(4).any(|bytes| bytes == b"av01"));
    }

    #[test]
    fn rgba_to_yuv444_preserves_per_pixel_chroma() {
        let mut image = image::RgbaImage::new(2, 1);
        image.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        image.put_pixel(1, 0, image::Rgba([0, 0, 255, 255]));

        let (_, u_plane, v_plane) = rgba_to_yuv444(&image);

        assert_eq!(u_plane.len(), 2);
        assert_eq!(v_plane.len(), 2);
        assert_ne!(u_plane[0], u_plane[1]);
        assert_ne!(v_plane[0], v_plane[1]);
    }
}
