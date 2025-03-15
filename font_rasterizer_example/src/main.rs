use font_rasterizer_example::run;

pub fn main() {
    env_logger::builder()
        .filter_module(module_path!(), log::LevelFilter::Debug)
        .filter_level(log::LevelFilter::Warn)
        .init();
    pollster::block_on(run());
}
