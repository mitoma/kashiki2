use std::{
    fs::{self, File},
    io::BufWriter,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use clap::{Parser, ValueEnum};
use font_collector::FontRepository;
use font_rasterizer::{
    color_theme::{ColorTheme, ThemedColor},
    context::WindowSize,
    glyph_vertex_buffer::Direction,
    rasterizer_pipeline::Quarity,
};
use image::{ImageBuffer, Rgba};
use serde_json::to_writer_pretty;
use stroke_parser::Action;
use ui_support::{
    Flags, InputResult, RenderData, SimpleStateCallback, SimpleStateSupport, generate_image_iter,
    layout_engine::{
        DebugModelNode, DebugWorldSnapshot, DefaultWorld, Model, ModelBorder, ModelOperation, World,
    },
    register_default_border, register_default_caret,
    ui::{SelectBox, SelectOption, SingleSvg, StackLayout, TextEdit},
    ui_context::{CharEasingsPreset, UiContext},
};
use winit::event::WindowEvent;

const FONT_DATA: &[u8] = include_bytes!("../../fonts/BIZUDMincho-Regular.ttf");
const EMOJI_FONT_DATA: &[u8] = include_bytes!("../../fonts/NotoEmoji-Regular.ttf");

#[derive(Debug, Clone, Parser)]
struct Args {
    #[arg(long, value_enum, default_value_t = CaseArg::All)]
    case: CaseArg,
    #[arg(long, default_value = "target/layout-debug")]
    output_dir: PathBuf,
    #[arg(long, default_value_t = 1280)]
    width: u32,
    #[arg(long, default_value_t = 960)]
    height: u32,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CaseArg {
    All,
    StackHorizontal,
    StackVertical,
    StackNested,
    TexteditHorizontal,
    TexteditVertical,
    WorldMixed,
    SvgAndSelectBox,
    InlineTextedit,
}

impl CaseArg {
    fn case_names(self) -> Vec<&'static str> {
        match self {
            Self::All => vec![
                "stack_horizontal",
                "stack_vertical",
                "stack_nested",
                "textedit_horizontal",
                "textedit_vertical",
                "world_mixed",
                "svg_and_select_box",
                "inline_textedit",
            ],
            Self::StackHorizontal => vec!["stack_horizontal"],
            Self::StackVertical => vec!["stack_vertical"],
            Self::StackNested => vec!["stack_nested"],
            Self::TexteditHorizontal => vec!["textedit_horizontal"],
            Self::TexteditVertical => vec!["textedit_vertical"],
            Self::WorldMixed => vec!["world_mixed"],
            Self::SvgAndSelectBox => vec!["svg_and_select_box"],
            Self::InlineTextedit => vec!["inline_textedit"],
        }
    }
}

type BuildCaseFn = fn(&UiContext, &mut DefaultWorld);

fn main() {
    env_logger::builder()
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .filter_level(log::LevelFilter::Info)
        .init();
    let args = Args::parse();
    pollster::block_on(run(args));
}

async fn run(args: Args) {
    fs::create_dir_all(&args.output_dir).unwrap();
    for case_name in args.case.case_names() {
        run_case(case_name, &args).await;
    }
}

async fn run_case(case_name: &str, args: &Args) {
    let output_dir = args.output_dir.join(case_name);
    fs::create_dir_all(&output_dir).unwrap();

    let window_size = WindowSize::new(args.width, args.height);
    let snapshot = Arc::new(Mutex::new(None));
    let callback = LayoutDebugCallback::new(window_size, case_name, Arc::clone(&snapshot));
    let support = SimpleStateSupport {
        window_icon: None,
        window_title: format!("layout_debug:{case_name}"),
        window_size,
        callback: Box::new(callback),
        quarity: Quarity::VeryHigh,
        color_theme: ColorTheme::SolarizedDark,
        flags: Flags::DEFAULT,
        font_repository: build_font_repository(),
        performance_mode: false,
        background_image: None,
        shader_art: None,
    };

    let image_iter = generate_image_iter(support, 100, web_time::Duration::from_millis(16)).await;
    let mut image = None;
    let mut frame_index = 0;
    for (img, idx) in image_iter {
        image = Some(img);
        frame_index = idx;
    }
    let image = image.expect("failed to render debug frame");
    let snapshot = snapshot
        .lock()
        .unwrap()
        .clone()
        .expect("debug snapshot was not captured");

    image
        .save(output_dir.join(format!("frame_{frame_index:04}.png")))
        .unwrap();

    overlay_debug_bounds(image, &snapshot)
        .save(output_dir.join(format!("frame_{frame_index:04}.overlay.png")))
        .unwrap();

    let writer = BufWriter::new(
        File::create(output_dir.join(format!("frame_{frame_index:04}.json"))).unwrap(),
    );
    to_writer_pretty(writer, &snapshot).unwrap();
}

fn build_font_repository() -> FontRepository {
    let mut font_repository = FontRepository::default();
    font_repository.add_fallback_font_from_binary(FONT_DATA.to_vec(), None);
    font_repository.add_fallback_font_from_binary(EMOJI_FONT_DATA.to_vec(), None);
    font_repository
}

struct LayoutDebugCallback {
    world: DefaultWorld,
    case_name: String,
    snapshot: Arc<Mutex<Option<DebugWorldSnapshot>>>,
}

impl LayoutDebugCallback {
    fn new(
        window_size: WindowSize,
        case_name: &str,
        snapshot: Arc<Mutex<Option<DebugWorldSnapshot>>>,
    ) -> Self {
        Self {
            world: DefaultWorld::new(window_size),
            case_name: case_name.to_owned(),
            snapshot,
        }
    }
}

impl SimpleStateCallback for LayoutDebugCallback {
    fn init(&mut self, context: &UiContext) {
        register_default_caret(context);
        register_default_border(context);
        build_case(&self.case_name)(context, &mut self.world);
        register_world_chars(context, &self.world);
        self.world
            .change_char_easings_preset(CharEasingsPreset::ZeroMotion);
        self.world.re_layout();
        self.world
            .look_current(ui_support::camera::CameraAdjustment::FitBothAndCentering);
    }

    fn resize(&mut self, window_size: WindowSize) {
        self.world.change_window_size(window_size);
    }

    fn update(&mut self, context: &UiContext) {
        self.world.update(context);
        self.world
            .look_current(ui_support::camera::CameraAdjustment::FitBothAndCentering);
    }

    fn input(&mut self, _context: &UiContext, _event: &WindowEvent) -> InputResult {
        InputResult::Noop
    }

    fn action(&mut self, _context: &UiContext, _action: stroke_parser::Action) -> InputResult {
        InputResult::Noop
    }

    fn render(&'_ mut self) -> RenderData<'_> {
        *self.snapshot.lock().unwrap() = Some(self.world.debug_snapshot());
        RenderData {
            camera: self.world.camera(),
            glyph_instances: self.world.glyph_instances(),
            vector_instances: self.world.vector_instances(),
            glyph_instances_for_modal: vec![],
            vector_instances_for_modal: vec![],
        }
    }

    fn shutdown(&mut self) {}
}

fn register_world_chars(context: &UiContext, world: &DefaultWorld) {
    context.register_string(world.current_string());
}

fn build_case(name: &str) -> BuildCaseFn {
    match name {
        "stack_horizontal" => build_stack_horizontal_case,
        "stack_vertical" => build_stack_vertical_case,
        "stack_nested" => build_stack_nested_case,
        "textedit_horizontal" => build_textedit_horizontal_case,
        "textedit_vertical" => build_textedit_vertical_case,
        "world_mixed" => build_world_mixed_case,
        "svg_and_select_box" => build_svg_and_select_box_case,
        "inline_textedit" => build_inline_textedit_case,
        _ => panic!("unknown case: {name}"),
    }
}

fn build_stack_horizontal_case(_context: &UiContext, world: &mut DefaultWorld) {
    let mut layout = StackLayout::new(Direction::Horizontal);
    layout.set_margin(0.6, 0.8);
    layout.add_model(Box::new(make_text_edit(
        "header\nstack horizontal",
        Direction::Horizontal,
        Some(14),
    )));
    layout.add_model(Box::new(make_text_edit(
        "body\nmultiple lines\nfor bounds",
        Direction::Horizontal,
        Some(12),
    )));
    layout.add_model(Box::new(make_text_edit(
        "footer",
        Direction::Horizontal,
        Some(16),
    )));
    world.add(Box::new(layout));
}

fn build_stack_vertical_case(_context: &UiContext, world: &mut DefaultWorld) {
    let mut layout = StackLayout::new(Direction::Vertical);
    layout.set_margin(1.0, 0.4);
    layout.add_model(Box::new(make_text_edit(
        "left panel",
        Direction::Horizontal,
        Some(20),
    )));
    layout.add_model(Box::new(make_text_edit(
        "center\nwrapped text",
        Direction::Horizontal,
        Some(10),
    )));
    layout.add_model(Box::new(make_text_edit(
        "right",
        Direction::Horizontal,
        Some(20),
    )));
    world.add(Box::new(layout));
}

fn build_stack_nested_case(_context: &UiContext, world: &mut DefaultWorld) {
    let mut root = StackLayout::new(Direction::Vertical);
    root.set_margin(0.8, 0.8);

    let mut upper = StackLayout::new(Direction::Horizontal);
    upper.set_margin(0.3, 0.5);
    upper.add_model(Box::new(make_text_edit(
        "left-top",
        Direction::Horizontal,
        Some(18),
    )));
    upper.add_model(Box::new(make_text_edit(
        "left-bottom\nwith two lines",
        Direction::Horizontal,
        Some(12),
    )));

    let mut lower = StackLayout::new(Direction::Horizontal);
    lower.set_margin(0.3, 0.5);
    lower.add_model(Box::new(make_text_edit(
        "縦書き確認",
        Direction::Vertical,
        Some(8),
    )));
    lower.add_model(Box::new(make_text_edit(
        "abc\ndef",
        Direction::Horizontal,
        Some(8),
    )));

    root.add_model(Box::new(upper));
    root.add_model(Box::new(lower));
    world.add(Box::new(root));
}

fn build_textedit_horizontal_case(_context: &UiContext, world: &mut DefaultWorld) {
    world.add(Box::new(make_text_edit(
        "horizontal text edit\nwith wrapping debug output\n1234567890",
        Direction::Horizontal,
        Some(12),
    )));
}

fn build_textedit_vertical_case(_context: &UiContext, world: &mut DefaultWorld) {
    world.add(Box::new(make_text_edit(
        "縦書き\nテキスト\n確認",
        Direction::Vertical,
        Some(8),
    )));
}

fn build_world_mixed_case(context: &UiContext, world: &mut DefaultWorld) {
    world.add(Box::new(make_text_edit(
        "world root A\nstack target",
        Direction::Horizontal,
        Some(12),
    )));

    let mut layout = StackLayout::new(Direction::Vertical);
    layout.set_margin(0.5, 0.5);
    layout.add_model(Box::new(make_text_edit(
        "nested left",
        Direction::Horizontal,
        Some(12),
    )));
    layout.add_model(Box::new(make_text_edit(
        "nested right",
        Direction::Horizontal,
        Some(12),
    )));
    world.add(Box::new(layout));

    let svg = SingleSvg::new(
        include_str!("../asset/kashikishi-icon-toon-flat.svg").to_string(),
        context,
        ThemedColor::Cyan,
    );
    world.add(Box::new(svg));
}

fn build_svg_and_select_box_case(context: &UiContext, world: &mut DefaultWorld) {
    let mut layout = StackLayout::new(context.global_direction());

    let logo = SingleSvg::new(
        include_str!("../../ui_support/asset/kashikishi-icon-toon-flat.svg").to_string(),
        context,
        ThemedColor::Blue,
    );
    layout.add_model(Box::new(logo));
    let logo = SingleSvg::new(
        include_str!("../../ui_support/asset/kashikishi-icon-toon-flat.svg").to_string(),
        context,
        ThemedColor::Red,
    );
    layout.add_model(Box::new(logo));

    let options = vec![
        SelectOption::new(
            "メモ帳を開く".to_string(),
            Action::new_command("mode", "category"),
        ),
        SelectOption::new(
            "ヘルプ(使い方の概説)を開く".to_string(),
            Action::new_command("mode", "help"),
        ),
        SelectOption::new(
            "炊紙を終了する".to_string(),
            Action::new_command("system", "exit"),
        ),
    ];
    let start_select =
        SelectBox::new_without_action_name(context, "炊紙 kashikishi".to_string(), options, None)
            .without_cancellable();
    layout.add_model(Box::new(start_select));
    layout.set_focus_model_index(2, false);
    world.add(Box::new(layout));
}

fn build_inline_textedit_case(context: &UiContext, world: &mut DefaultWorld) {
    let textedit = make_text_edit(
        "inline textedit\nwith multiple lines",
        Direction::Horizontal,
        Some(30),
    );
    world.add(Box::new(textedit));
    world.look_next(ui_support::camera::CameraAdjustment::FitBoth);
    world.editor_operation(&text_buffer::action::EditorOperation::Head);
    world.model_operation(&ModelOperation::SetPreedit(Some((
        "ほげほげ".to_string(),
        None,
    ))));
    context.register_string("[ほげほげ]".to_string());
    world.re_layout();
}

fn make_text_edit(text: &str, direction: Direction, max_col: Option<usize>) -> TextEdit {
    let mut text_edit = TextEdit::default();
    text_edit.model_operation(&ModelOperation::ToggleMinBound);
    text_edit.model_operation(&ModelOperation::ChangeDirection(Some(direction)));
    if let Some(max_col) = max_col {
        text_edit.model_operation(&ModelOperation::SetMaxCol(max_col));
    }
    text_edit.model_operation(&ModelOperation::SetModelBorder(ModelBorder::Square));
    text_edit.editor_operation(&text_buffer::action::EditorOperation::InsertString(
        text.to_owned(),
    ));
    text_edit
}

fn overlay_debug_bounds(
    mut image: ImageBuffer<Rgba<u8>, Vec<u8>>,
    snapshot: &DebugWorldSnapshot,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    for (index, node) in snapshot.models.iter().enumerate() {
        draw_node(&mut image, node, 0, index);
    }
    for (index, node) in snapshot.modal_models.iter().enumerate() {
        draw_node(&mut image, node, 1, index);
    }
    image
}

fn draw_node(
    image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    node: &DebugModelNode,
    depth: usize,
    ordinal: usize,
) {
    let color = color_for(depth, ordinal, node.name);
    let quad = node
        .projected_quad_ndc
        .map(|point| ndc_to_pixel(point, image.width(), image.height()));
    for edge in 0..quad.len() {
        draw_line(image, quad[edge], quad[(edge + 1) % quad.len()], color);
    }
    draw_cross(
        image,
        ndc_to_pixel(node.projected_center_ndc, image.width(), image.height()),
        color,
    );
    for (index, child) in node.children.iter().enumerate() {
        draw_node(image, child, depth + 1, index);
    }
}

fn color_for(depth: usize, ordinal: usize, name: &str) -> Rgba<u8> {
    let palette = [
        Rgba([239, 83, 80, 220]),
        Rgba([66, 165, 245, 220]),
        Rgba([255, 238, 88, 220]),
        Rgba([102, 187, 106, 220]),
        Rgba([255, 167, 38, 220]),
        Rgba([171, 71, 188, 220]),
    ];
    let mut index = depth + ordinal;
    if name == "TextEdit" {
        index += 1;
    }
    palette[index % palette.len()]
}

fn ndc_to_pixel(point: [f32; 2], width: u32, height: u32) -> (i32, i32) {
    let x = ((point[0] + 1.0) * 0.5 * (width.saturating_sub(1)) as f32).round() as i32;
    let y = ((1.0 - (point[1] + 1.0) * 0.5) * (height.saturating_sub(1)) as f32).round() as i32;
    (x, y)
}

fn draw_cross(image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, center: (i32, i32), color: Rgba<u8>) {
    for delta in -3..=3 {
        blend_pixel(image, center.0 + delta, center.1, color);
        blend_pixel(image, center.0, center.1 + delta, color);
    }
}

fn draw_line(
    image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    (x0, y0): (i32, i32),
    (x1, y1): (i32, i32),
    color: Rgba<u8>,
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let (mut x, mut y) = (x0, y0);

    loop {
        blend_pixel(image, x, y, color);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

fn blend_pixel(image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, x: i32, y: i32, color: Rgba<u8>) {
    if x < 0 || y < 0 || x >= image.width() as i32 || y >= image.height() as i32 {
        return;
    }
    let pixel = image.get_pixel_mut(x as u32, y as u32);
    let alpha = color[3] as f32 / 255.0;
    for channel in 0..3 {
        pixel[channel] = ((pixel[channel] as f32 * (1.0 - alpha)) + (color[channel] as f32 * alpha))
            .round() as u8;
    }
    pixel[3] = 255;
}
