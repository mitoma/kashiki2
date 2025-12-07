mod acrion_record_converter;
mod args;
mod callback;

use std::{
    io::{Cursor, Write},
    time::Duration,
};

use apng::{Config, Encoder, load_dynamic_image};
use clap::ValueEnum;
use font_collector::{FontCollector, FontRepository};
use font_rasterizer::{color_theme::ColorTheme, context::WindowSize, rasterizer_pipeline::Quarity};
use log::info;
use ui_support::{Flags, SimpleStateSupport, generate_image_iter, ui_context::CharEasingsPreset};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub use crate::args::{Args, CharEasingsPresetArg, ColorThemeArg};
use crate::{
    acrion_record_converter::ActionRecordConverter, args::WindowSizeArg, callback::Callback,
};

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
pub async fn run_wasm(
    target_string: &str,
    window_size: &str,
    color_theme: &str,
    easing_preset: &str,
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
    run(target_string, window_size, color_theme, easing_preset).await
}

pub async fn run_native(
    target_string: &str,
    window_size: WindowSize,
    color_theme: ColorTheme,
    easing_preset: CharEasingsPreset,
) {
    let result = run(target_string, window_size, color_theme, easing_preset).await;
    let mut file = std::fs::File::create("output.png").unwrap();
    file.write_all(&result).unwrap();
}

pub async fn run(
    target_string: &str,
    window_size: WindowSize,
    color_theme: ColorTheme,
    easing_preset: CharEasingsPreset,
) -> Vec<u8> {
    let mut repository = ActionRecordConverter::new();
    repository.set_direction_vertical();
    repository.append(target_string);

    let fps = 24;
    let sec = repository.all_time_frames().as_secs() as u32 + 2;

    let mut font_repository = FontRepository::default();
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
        flags: Flags::DEFAULT,
        font_repository,
        performance_mode: false,
    };
    log::info!("support initialized.");

    let mut writer: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    let frame = apng::Frame {
        delay_num: Some(1),
        delay_den: Some(fps as u16),
        ..Default::default()
    };
    log::info!("frame initialized.");

    let mut image_iter =
        generate_image_iter(support, fps * sec, Duration::from_millis(1000 / fps as u64))
            .await
            .map(|(image, index)| {
                let dynimage = image::DynamicImage::ImageRgba8(image);
                let png_image = load_dynamic_image(dynimage).unwrap();
                (png_image, index)
            });
    log::info!("image iterator created.");
    let (image, _idx) = image_iter.next().unwrap();
    log::info!("get first frame.");

    let config = Config {
        width: window_size.width,
        height: window_size.height,
        num_frames: fps * sec,
        num_plays: 1,
        color: image.color_type,
        depth: image.bit_depth,
        filter: png::FilterType::NoFilter,
    };
    let mut encoder = Encoder::new(&mut writer, config).unwrap();
    encoder.write_frame(&image, frame.clone()).unwrap();

    for (png_image, idx) in image_iter {
        info!("encoding... frame: {}/{}", idx + 1, fps * sec);
        encoder.write_frame(&png_image, frame.clone()).unwrap();
    }
    encoder.finish_encode().unwrap();
    info!("finish!");
    writer.into_inner()
}
