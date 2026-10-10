//! FontVertexConverter の動作確認用サンプル。
//! 指定したフォントと文字の有向二次曲線を image クレートのみで描画し、
//! 始点・制御点・終点を色分けして PNG に書き出す。

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use font_collector::{FontCollector, FontData};
use font_rasterizer::{
    VectorVertex,
    char_width_calcurator::{CharWidth, CharWidthCalculator},
    font_converter::convert_char_to_vector_vertices,
};
use image::{Rgba, RgbaImage};

const FONT_DATA: &[u8] = include_bytes!("../../fonts/BIZUDMincho-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");

#[derive(Parser, Debug, Clone)]
struct Args {
    /// png に書き出す対象の文字（複数指定可。例: "あ愛A" のように連続で指定）
    #[arg(short, long, default_value = "あ")]
    chars: String,

    /// システムフォント名（例: "Noto Sans JP"）。省略時は同梱フォントのみを使う
    #[arg(short, long)]
    font_name: Option<String>,

    /// 独自のフォントファイルパス（.ttf/.otf, 複数指定可）
    #[arg(long)]
    font_path: Vec<PathBuf>,

    /// ASCII 文字を上書きするシステムフォント名
    #[arg(long)]
    ascii_override_font_name: Option<String>,

    #[arg(long, default_value_t = 512)]
    width: u32,

    #[arg(long, default_value_t = 512)]
    height: u32,

    /// 頂点を表す点の半径（ピクセル）
    #[arg(long, default_value_t = 5)]
    point_radius: i32,

    /// 出力先ディレクトリ
    #[arg(short, long, default_value = "target/font_char_to_png")]
    output_dir: PathBuf,
}

fn draw_line(img: &mut RgbaImage, from: (i32, i32), to: (i32, i32), color: Rgba<u8>) {
    let (mut x, mut y) = from;
    let (target_x, target_y) = to;
    let delta_x = (target_x - x).abs();
    let step_x = if x < target_x { 1 } else { -1 };
    let delta_y = -(target_y - y).abs();
    let step_y = if y < target_y { 1 } else { -1 };
    let mut error = delta_x + delta_y;

    loop {
        if x >= 0 && y >= 0 && x < img.width() as i32 && y < img.height() as i32 {
            img.put_pixel(x as u32, y as u32, color);
        }
        if x == target_x && y == target_y {
            break;
        }
        let doubled_error = error * 2;
        if doubled_error >= delta_y {
            error += delta_y;
            x += step_x;
        }
        if doubled_error <= delta_x {
            error += delta_x;
            y += step_y;
        }
    }
}

fn draw_filled_circle(img: &mut RgbaImage, cx: i32, cy: i32, radius: i32, color: Rgba<u8>) {
    let (width, height) = (img.width() as i32, img.height() as i32);
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            let (x, y) = (cx + dx, cy + dy);
            if x >= 0 && x < width && y >= 0 && y < height {
                img.put_pixel(x as u32, y as u32, color);
            }
        }
    }
}

/// 有向二次曲線と制御点を、フィルは行わず image クレートのみで描画する
fn render_curve_debug(
    vector_vertex: &VectorVertex,
    width: u32,
    height: u32,
    point_radius: i32,
) -> RgbaImage {
    let mut img = RgbaImage::from_pixel(width, height, Rgba([250, 250, 250, 255]));

    let curves = vector_vertex.curves();
    if curves.is_empty() {
        return img;
    }

    let (mut min_x, mut max_x, mut min_y, mut max_y) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for [x, y] in curves
        .iter()
        .flat_map(|curve| [curve.start, curve.control, curve.end])
    {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    // 座標が 1 点しかない場合などでも span が 0 にならないようにする
    let span = (max_x - min_x).max(max_y - min_y).max(f32::EPSILON);
    let span = span * 1.2; // 余白として 1 割ずつ広げる
    let center_x = (min_x + max_x) / 2.0;
    let center_y = (min_y + max_y) / 2.0;
    let scale = (width.min(height) as f32) / span;

    let to_pixel = |x: f32, y: f32| -> (i32, i32) {
        let px = (x - center_x) * scale + width as f32 / 2.0;
        // font 座標系は Y 軸上向きなので画像座標系（Y 軸下向き）へ反転する
        let py = (center_y - y) * scale + height as f32 / 2.0;
        (px.round() as i32, py.round() as i32)
    };

    for curve in curves {
        let start = to_pixel(curve.start[0], curve.start[1]);
        let control = to_pixel(curve.control[0], curve.control[1]);
        let end = to_pixel(curve.end[0], curve.end[1]);
        draw_line(&mut img, start, control, Rgba([220, 160, 170, 255]));
        draw_line(&mut img, control, end, Rgba([220, 160, 170, 255]));
        let mut previous = start;
        for sample in 1..=64 {
            let parameter = sample as f32 / 64.0;
            let complement = 1.0 - parameter;
            let point: [f32; 2] = std::array::from_fn(|axis| {
                complement * complement * curve.start[axis]
                    + 2.0 * complement * parameter * curve.control[axis]
                    + parameter * parameter * curve.end[axis]
            });
            let current = to_pixel(point[0], point[1]);
            draw_line(&mut img, previous, current, Rgba([0, 128, 128, 255]));
            previous = current;
        }
    }

    for curve in curves {
        for (point, color) in [
            (curve.start, Rgba([34, 139, 34, 255])),
            (curve.control, Rgba([220, 20, 60, 255])),
            (curve.end, Rgba([30, 144, 255, 255])),
        ] {
            let (px, py) = to_pixel(point[0], point[1]);
            draw_filled_circle(&mut img, px, py, point_radius, color);
        }
    }

    img
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .init();

    let args = Args::parse();

    let mut font_collector = FontCollector::default();
    font_collector.add_system_fonts();

    let mut fonts: Vec<FontData> = Vec::new();
    for path in &args.font_path {
        let data = fs::read(path)?;
        let font_data = font_collector
            .convert_font(data, None)
            .ok_or_else(|| format!("failed to load font file: {:?}", path))?;
        fonts.push(font_data);
    }
    if let Some(font_name) = &args.font_name {
        let font_data = font_collector
            .load_font(font_name)
            .ok_or_else(|| format!("system font not found: {}", font_name))?;
        fonts.push(font_data);
    }
    // 常にフォールバックとして同梱フォントを使う
    fonts.push(
        font_collector
            .convert_font(FONT_DATA.to_vec(), None)
            .unwrap(),
    );
    fonts.push(
        font_collector
            .convert_font(EMOJI_FONT_DATA.to_vec(), None)
            .unwrap(),
    );

    let ascii_override_font = args
        .ascii_override_font_name
        .as_ref()
        .map(|name| {
            font_collector
                .load_font(name)
                .ok_or_else(|| format!("system font not found: {}", name))
        })
        .transpose()?;

    let fonts = Arc::new(fonts);
    let char_width_calculator = CharWidthCalculator::new(fonts.clone());

    fs::create_dir_all(&args.output_dir)?;

    for c in args.chars.chars() {
        let width: CharWidth = char_width_calculator.get_width(c);
        let (h_vertex, v_vertex) =
            convert_char_to_vector_vertices(fonts.clone(), ascii_override_font.clone(), c, width)?;

        let h_path = args.output_dir.join(format!("char_{c}_horizontal.png"));
        render_curve_debug(&h_vertex, args.width, args.height, args.point_radius).save(&h_path)?;
        println!("Generated: {:?}", h_path);

        if let Some(v_vertex) = v_vertex {
            let v_path = args.output_dir.join(format!("char_{c}_vertical.png"));
            render_curve_debug(&v_vertex, args.width, args.height, args.point_radius)
                .save(&v_path)?;
            println!("Generated: {:?}", v_path);
        }
    }

    Ok(())
}
