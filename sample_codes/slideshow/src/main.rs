use slideshow::run;

pub fn main() {
    std::env::set_var("RUST_LOG", "slideshow=info");
    env_logger::init();
    pollster::block_on(run());
}
