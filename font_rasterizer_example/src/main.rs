use font_rasterizer_example::run;

pub fn main() {
    std::env::set_var("RUST_LOG", "support_test=debug");
    pollster::block_on(run());
}
