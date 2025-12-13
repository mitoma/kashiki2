use clap::Parser;
use oogiri_gen::{Args, run_native};

fn main() {
    let args = Args::parse();
    env_logger::builder()
        .filter_module(module_path!(), log::LevelFilter::Debug)
        .filter_level(log::LevelFilter::Warn)
        .init();
    pollster::block_on(run_native(
        args.target_string.as_str(),
        args.window_size.into(),
        args.color_theme.into(),
        args.preset.into(),
        24,
    ));
}
