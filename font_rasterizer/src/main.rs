use font_rusterizer::run;

fn main() {
    std::env::set_var("RUST_LOG", "font_rusterizer=debug");
    pollster::block_on(run());
}
