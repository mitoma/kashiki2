//! FontVertexConverter の動作確認用サンプル。
//! 指定したフォントと文字のグリフ頂点座標を、GPU を使わず image クレートのみで
//! 中心点・制御点・区間始点・区間終点・輪郭点を色分けして PNG に書き出す。

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use font_collector::{FontCollector, FontData};
use font_rasterizer::{
    VectorVertex, VertexPointKind,
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

fn point_color(kind: VertexPointKind) -> Rgba<u8> {
    match kind {
        VertexPointKind::Center => Rgba([230, 200, 20, 255]), // 黄: 中心点
        VertexPointKind::Control => Rgba([220, 20, 60, 255]), // 赤: 制御点
        VertexPointKind::SegmentStart => Rgba([34, 139, 34, 255]), // 緑: 区間始点
        VertexPointKind::SegmentEnd => Rgba([30, 144, 255, 255]), // 青: 区間終点
        VertexPointKind::OnCurve => Rgba([255, 140, 0, 255]), // 橙: 輪郭点
    }
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

fn relationship_line_color(first: VertexPointKind, second: VertexPointKind) -> Option<Rgba<u8>> {
    use VertexPointKind::{Center, Control, SegmentEnd, SegmentStart};

    match (first, second) {
        (Center, SegmentStart | SegmentEnd) | (SegmentStart | SegmentEnd, Center) => {
            Some(Rgba([128, 0, 128, 255]))
        }
        (Control, SegmentStart | SegmentEnd) | (SegmentStart | SegmentEnd, Control) => {
            Some(Rgba([220, 20, 60, 255]))
        }
        (SegmentStart, SegmentEnd) | (SegmentEnd, SegmentStart) => Some(Rgba([0, 128, 128, 255])),
        _ => None,
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

/// VectorVertex の各頂点座標を、フィルは行わず点として image クレートのみで描画する
fn render_vector_vertex_debug_points(
    vector_vertex: &VectorVertex,
    width: u32,
    height: u32,
    point_radius: i32,
) -> RgbaImage {
    let mut img = RgbaImage::from_pixel(width, height, Rgba([250, 250, 250, 255]));

    let points = vector_vertex.debug_points();
    if points.is_empty() {
        return img;
    }

    let (mut min_x, mut max_x, mut min_y, mut max_y) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for ([x, y], _) in &points {
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
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

    for triangle in vector_vertex.debug_triangles() {
        for &(first_index, second_index) in &[(0, 1), (1, 2), (2, 0)] {
            let (first_pos, first_kind) = triangle[first_index];
            let (second_pos, second_kind) = triangle[second_index];
            if let Some(color) = relationship_line_color(first_kind, second_kind) {
                draw_line(
                    &mut img,
                    to_pixel(first_pos[0], first_pos[1]),
                    to_pixel(second_pos[0], second_pos[1]),
                    color,
                );
            }
        }
    }

    for (pos, kind) in points {
        let (px, py) = to_pixel(pos[0], pos[1]);
        draw_filled_circle(&mut img, px, py, point_radius, point_color(kind));
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
        render_vector_vertex_debug_points(&h_vertex, args.width, args.height, args.point_radius)
            .save(&h_path)?;
        println!("Generated: {:?}", h_path);

        if let Some(v_vertex) = v_vertex {
            let v_path = args.output_dir.join(format!("char_{c}_vertical.png"));
            render_vector_vertex_debug_points(
                &v_vertex,
                args.width,
                args.height,
                args.point_radius,
            )
            .save(&v_path)?;
            println!("Generated: {:?}", v_path);
        }
    }

    Ok(())
}
