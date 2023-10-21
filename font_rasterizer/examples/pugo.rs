pub fn main() {
    std::env::set_var("RUST_LOG", "font_rasterizer=debug");
    env_logger::init();

    pollster::block_on(font_rasterizer::support::debug::run());
}
