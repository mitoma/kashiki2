use font_rasterizer::{
    VectorVertexBuilder,
    rasterizer_renderrer::OutlineFillRule,
    vector_vertex_png_renderer::{VectorVertexPngRendererOptions, render_vector_vertex_to_png},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = VectorVertexBuilder::new();
    builder.move_to(-0.7, -0.7);
    builder.line_to(-0.7, 0.7);
    builder.line_to(0.7, 0.7);
    builder.line_to(0.7, -0.7);
    builder.close();
    let vertex = builder.build();

    let options = VectorVertexPngRendererOptions {
        width: 128,
        height: 128,
        outline_fill_rule: OutlineFillRule::NonZero,
        enable_antialiasing: true,
        foreground_color: [1.0, 1.0, 1.0],
        background_color: [0, 0, 0, 0],
    };

    render_vector_vertex_to_png(vertex, "target/vector_vertex_test.png", options)?;
    Ok(())
}
