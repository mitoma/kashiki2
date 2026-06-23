use font_rasterizer::{
    VectorVertexBuilder,
    rasterizer_renderrer::OutlineFillRule,
    vector_vertex_png_renderer::{VectorVertexPngRendererOptions, render_vector_vertex_to_png},
};
use chrono::Local;
use serde::Deserialize;
use std::path::Path;
use std::fs;

#[derive(Deserialize)]
struct VectorTestCase {
    case_name: String,
    #[serde(default)]
    renderer_option: RendererOptions,
    vertex: VertexDef,
}

#[derive(Deserialize, Default)]
struct RendererOptions {
    #[serde(default = "default_width")]
    width: u32,
    #[serde(default = "default_height")]
    height: u32,
    #[serde(default = "default_foreground_color")]
    foreground_color: [f32; 3],
    #[serde(default = "default_background_color")]
    background_color: [u8; 4],
    #[serde(default)]
    enable_antialiasing: bool,
}

fn default_width() -> u32 {
    1024
}

fn default_height() -> u32 {
    1024
}

fn default_foreground_color() -> [f32; 3] {
    [0.14, 0.18, 0.24]
}

fn default_background_color() -> [u8; 4] {
    [245, 242, 232, 255]
}

#[derive(Deserialize)]
struct VertexDef {
    path: String,
}

fn parse_svg_path(path_data: &str) -> Result<VectorVertexBuilder, Box<dyn std::error::Error>> {
    let mut builder = VectorVertexBuilder::new();
    let mut tokens = path_data.split_whitespace();
    let mut current_x;
    let mut current_y;

    while let Some(cmd) = tokens.next() {
        match cmd {
            "M" => {
                current_x = tokens.next().ok_or("Missing M x")?.parse()?;
                current_y = tokens.next().ok_or("Missing M y")?.parse()?;
                builder.move_to(current_x, current_y);
            }
            "L" => {
                current_x = tokens.next().ok_or("Missing L x")?.parse()?;
                current_y = tokens.next().ok_or("Missing L y")?.parse()?;
                builder.line_to(current_x, current_y);
            }
            "Q" => {
                let cx = tokens.next().ok_or("Missing Q cx")?.parse()?;
                let cy = tokens.next().ok_or("Missing Q cy")?.parse()?;
                current_x = tokens.next().ok_or("Missing Q x")?.parse()?;
                current_y = tokens.next().ok_or("Missing Q y")?.parse()?;
                builder.quad_to(cx, cy, current_x, current_y);
            }
            "C" => {
                let cx1 = tokens.next().ok_or("Missing C cx1")?.parse()?;
                let cy1 = tokens.next().ok_or("Missing C cy1")?.parse()?;
                let cx2 = tokens.next().ok_or("Missing C cx2")?.parse()?;
                let cy2 = tokens.next().ok_or("Missing C cy2")?.parse()?;
                current_x = tokens.next().ok_or("Missing C x")?.parse()?;
                current_y = tokens.next().ok_or("Missing C y")?.parse()?;
                builder.curve_to(cx1, cy1, cx2, cy2, current_x, current_y);
            }
            "Z" | "z" => {
                builder.close();
            }
            _ => {
                // Skip unknown or numeric tokens
            }
        }
    }

    Ok(builder)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder().is_test(true).try_init().ok();
    let result_dir_name = format!("target/test_results_{}", Local::now().format("%Y%m%d_%H%M%S"));

    let cases_dir = Path::new("font_rasterizer/examples/cases");
    if !cases_dir.exists() {
        fs::create_dir_all(cases_dir)?;
    }

    let mut processed = 0;
    for entry in fs::read_dir(cases_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|ext| ext == "toml") {
            if let Err(e) = process_case(&path, &result_dir_name) {
                eprintln!("Error processing {:?}: {}", path, e);
                continue;
            }
            processed += 1;
        }
    }

    println!("Successfully processed {} test case(s)", processed);

    Ok(())
}

fn process_case(path: &Path, result_dir_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let case: VectorTestCase = toml::from_str(&content)?;

    let builder = parse_svg_path(&case.vertex.path)?;
    let vertex = builder.build();

    let options = VectorVertexPngRendererOptions {
        width: case.renderer_option.width,
        height: case.renderer_option.height,
        outline_fill_rule: OutlineFillRule::NonZero,
        enable_antialiasing: case.renderer_option.enable_antialiasing,
        foreground_color: case.renderer_option.foreground_color,
        background_color: case.renderer_option.background_color,
    };

    let output_path = format!("{}/{}.png", result_dir_name, case.case_name);
    render_vector_vertex_to_png(vertex, &output_path, options)?;
    println!("Generated: {}", output_path);

    Ok(())
}
