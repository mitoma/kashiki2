mod callback;

use std::time::Duration;

use apng::{ParallelEncoder, load_dynamic_image};
use font_collector::FontRepository;
use font_rasterizer::{color_theme::ColorTheme, context::WindowSize, rasterizer_pipeline::Quarity};
use log::info;
use ui_support::{Flags, SimpleStateSupport, generate_image_iter};

use crate::callback::Callback;

const FONT_DATA: &[u8] = include_bytes!("../../../fonts/BIZUDMincho-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] = include_bytes!("../../../fonts/NotoEmoji-Regular.ttf");

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    let mut font_repository = FontRepository::default();
    font_repository.add_fallback_font_from_binary(FONT_DATA.to_vec(), None);
    font_repository.add_fallback_font_from_binary(EMOJI_FONT_DATA.to_vec(), None);

    let mut senario = std::collections::BTreeMap::new();
    senario.insert(
        0,
        vec![text_buffer::action::EditorOperation::InsertString(
            "こんにちは".into(),
        )],
    );
    senario.insert(10, vec![text_buffer::action::EditorOperation::InsertEnter]);
    senario.insert(10, vec![text_buffer::action::EditorOperation::InsertEnter]);
    senario.insert(
        20,
        vec![text_buffer::action::EditorOperation::InsertString(
            "ぽげぽげほげ".to_string(),
        )],
    );
    senario.insert(
        50,
        vec![text_buffer::action::EditorOperation::InsertChar('🐖')],
    );
    senario.insert(
        80,
        vec![text_buffer::action::EditorOperation::InsertChar('🐖')],
    );
    senario.insert(
        100,
        vec![text_buffer::action::EditorOperation::InsertChar('🐖')],
    );

    let window_size = WindowSize::new(600, 600);
    let callback = Callback::new(window_size, senario);
    let support = SimpleStateSupport {
        window_icon: None,
        window_title: "Hello".to_string(),
        window_size,
        callback: Box::new(callback),
        quarity: Quarity::VeryHigh,
        color_theme: ColorTheme::SolarizedLight,
        flags: Flags::DEFAULT,
        font_repository,
        performance_mode: false,
    };
    let fps = 24;
    let sec = 10;

    let path = std::path::Path::new("test-animation.png");
    let frame = apng::Frame {
        delay_num: Some(1),
        delay_den: Some(fps as u16),
        ..Default::default()
    };

    let mut image_iter = generate_image_iter(support, fps * sec, Duration::from_millis(10u64))
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
