use slideshow::run;

pub fn main() {
    std::env::set_var("RUST_LOG", "slideshow=info");
    //std::env::set_var("FONT_RASTERIZER_DEBUG", "debug");
    env_logger::init();
    pollster::block_on(run());
}
