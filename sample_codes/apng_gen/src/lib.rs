mod acrion_record_converter;
mod args;
mod callback;

use std::{
    io::{Cursor, Write},
    sync::{Arc, Mutex},
    time::Duration,
};

use apng::{Config, Encoder, load_dynamic_image};
#[cfg(target_arch = "wasm32")]
use clap::ValueEnum;
use font_collector::FontRepository;
use font_rasterizer::{color_theme::ColorTheme, context::WindowSize, rasterizer_pipeline::Quarity};
use log::info;
use ui_support::{Flags, SimpleStateSupport, generate_images, ui_context::CharEasingsPreset};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use js_sys::Uint8Array;

#[cfg(target_arch = "wasm32")]
use crate::args::WindowSizeArg;
pub use crate::args::{Args, CharEasingsPresetArg, ColorThemeArg};
use crate::{acrion_record_converter::ActionRecordConverter, callback::Callback};

const FONT_DATA: &[u8] = include_bytes!("../../../fonts/BIZUDMincho-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] = include_bytes!("../../../fonts/NotoEmoji-Regular.ttf");

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
    
    run(
        target_string,
        window_size,
        color_theme,
        easing_preset,
        fps_num,
        transparent_bg,
        font_binary_vec,
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

pub async fn run_native(
    target_string: &str,
    window_size: WindowSize,
    color_theme: ColorTheme,
    easing_preset: CharEasingsPreset,
    fps: u32,
    transparent_bg: bool,
) {
    let result = run(
        target_string,
        window_size,
        color_theme,
        easing_preset,
        fps,
        transparent_bg,
        None,
        None,
    )
    .await;
    let mut file = std::fs::File::create("target/output.png").unwrap();
    file.write_all(&result).unwrap();
}

pub async fn run(
    target_string: &str,
    window_size: WindowSize,
    color_theme: ColorTheme,
    easing_preset: CharEasingsPreset,
    fps: u32,
    transparent_bg: bool,
    font_binary: Option<Vec<u8>>,
    per_frame_callback: Option<Box<dyn Fn(u32, u32) + Send + 'static>>,
) -> Vec<u8> {
    let mut repository = ActionRecordConverter::new();
    repository.set_direction_vertical();
    repository.append(target_string);

    let sec = repository.all_time_frames().as_secs() as u32 + 2;

    let mut font_repository = FontRepository::default();
    // Add custom font if provided
    if let Some(font_data) = font_binary {
        font_repository.add_fallback_font_from_binary(font_data, None);
    }
    // Always add default fonts as fallback
    font_repository.add_fallback_font_from_binary(FONT_DATA.to_vec(), None);
    font_repository.add_fallback_font_from_binary(EMOJI_FONT_DATA.to_vec(), None);
    log::info!("font_repository initialized.");
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
    };
    log::info!("support initialized.");

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
