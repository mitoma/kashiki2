mod acrion_record_converter;
mod args;
mod callback;

use std::time::Duration;

use apng::{ParallelEncoder, load_dynamic_image};
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

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub async fn run_wasm(
    target_string: &str,
    window_size: &str,
    color_theme: &str,
    easing_preset: &str,
) {
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

pub async fn run(
    target_string: &str,
    window_size: WindowSize,
    color_theme: ColorTheme,
    easing_preset: CharEasingsPreset,
) {
    let mut repository = ActionRecordConverter::new();
    repository.set_direction_vertical();
    repository.append(target_string);

    let fps = 24;
    let sec = repository.all_time_frames().as_secs() as u32 + 2;

    let mut font_collector = FontCollector::default();
    font_collector.add_system_fonts();
    let mut font_repository = FontRepository::new(font_collector);
    [
        "Yuji Syuku Regular",
        "BIZ UDゴシック",
        "UD デジタル 教科書体 N",
        "UD デジタル 教科書体 N-R",
    ]
    .iter()
    .for_each(|name| {
        font_repository.add_fallback_font_from_system(name);
    });
    font_repository.add_fallback_font_from_binary(FONT_DATA.to_vec(), None);
    font_repository.add_fallback_font_from_binary(EMOJI_FONT_DATA.to_vec(), None);

    let callback = Callback::new(window_size, Box::new(repository), easing_preset);
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

    let path = std::path::Path::new("test-animation.png");
    let frame = apng::Frame {
        delay_num: Some(1),
        delay_den: Some(fps as u16),
        ..Default::default()
    };

    let mut image_iter =
        generate_image_iter(support, fps * sec, Duration::from_millis(1000 / fps as u64))
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
        fps * sec,
        Some(1),
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
