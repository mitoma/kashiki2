use std::{
    panic::AssertUnwindSafe,
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::Args as ClapArgs;
use font_collector::{FontCollector, FontRepository};
use font_rasterizer::{context::WindowSize, rasterizer_pipeline::Quarity};
use stroke_parser::Action;
use thiserror::Error;
use ui_support::{
    Flags, SimpleStateSupport,
    action_recorder::{
        ActionRecordError, ActionRecorder, InMemoryActionRecordRepository, as_snapshot_name,
        load_action_records,
    },
    headless::{HeadlessExit, HeadlessOptions, Snapshot, run_headless},
    run_support,
};
use web_time::Duration;

use crate::{EMOJI_FONT_DATA, FONT_DATA, KashikishiCallback, kashikishi_config::KashikishiConfig};

const EXIT_SCENARIO_ERROR: u8 = 1;
const EXIT_TIMEOUT: u8 = 2;
const EXIT_PANIC: u8 = 3;

#[derive(ClapArgs, Debug, Clone)]
pub struct TestArgs {
    /// scenario file (JSON Lines of actions)
    #[arg(long)]
    pub scenario: PathBuf,

    /// output directory of snapshots [default: target/e2e/<scenario name>]
    #[arg(long)]
    pub snapshot_dir: Option<PathBuf>,

    /// directory for config and memos [default: a temporary directory removed after the run]
    #[arg(long)]
    pub data_dir: Option<PathBuf>,

    /// initial config file (JSON) [default: default config]
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// rendering size (WIDTHxHEIGHT)
    #[arg(long, default_value = "800x600", value_parser = parse_window_size)]
    pub window_size: WindowSize,

    /// milliseconds to advance the fixed clock per frame
    #[arg(long, default_value_t = 16)]
    pub frame_ms: u32,

    /// fail as timeout when the scenario does not finish within this number of frames
    #[arg(long, default_value_t = 36000)]
    pub max_frames: u32,

    /// replay in a window in real time to watch it (no snapshots; --snapshot-dir, --frame-ms and --max-frames are ignored)
    #[arg(long)]
    pub headed: bool,
}

fn parse_window_size(value: &str) -> Result<WindowSize, String> {
    let (width, height) = value
        .split_once('x')
        .ok_or_else(|| format!("invalid window size: {value} (expected WIDTHxHEIGHT)"))?;
    let width = width.parse::<u32>().map_err(|err| err.to_string())?;
    let height = height.parse::<u32>().map_err(|err| err.to_string())?;
    if width == 0 || height == 0 {
        return Err(format!("invalid window size: {value}"));
    }
    Ok(WindowSize::new(width, height))
}

#[derive(Debug, Error)]
enum ScenarioError {
    #[error(transparent)]
    Load(#[from] ActionRecordError),
    #[error("invalid snapshot name {name:?} (allowed characters: A-Z a-z 0-9 _ -)")]
    InvalidSnapshotName { name: String },
    #[error("failed to read config {path}: {message}")]
    Config { path: PathBuf, message: String },
    #[error("failed to prepare directory {path}: {source}")]
    Directory {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to save snapshot {path}: {message}")]
    SaveSnapshot { path: PathBuf, message: String },
}

pub(crate) fn run_test(test_args: TestArgs) -> ExitCode {
    let data_dir = match prepare_data_dir(test_args.data_dir.as_deref()) {
        Ok(data_dir) => data_dir,
        Err(err) => return report_error(err),
    };
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| run_scenario(&test_args)));
    if data_dir.temporary {
        let _ = std::fs::remove_dir_all(&data_dir.path);
    }
    match result {
        Ok(Ok(ScenarioOutcome::Headed)) => ExitCode::SUCCESS,
        Ok(Ok(ScenarioOutcome::Headless(HeadlessExit::Exited { frames }))) => {
            eprintln!("scenario finished: {frames} frames");
            ExitCode::SUCCESS
        }
        Ok(Ok(ScenarioOutcome::Headless(HeadlessExit::Timeout { frames }))) => {
            eprintln!("scenario timeout: not finished within {frames} frames");
            ExitCode::from(EXIT_TIMEOUT)
        }
        Ok(Err(err)) => report_error(err),
        Err(_) => {
            eprintln!("scenario panicked");
            ExitCode::from(EXIT_PANIC)
        }
    }
}

fn report_error(err: ScenarioError) -> ExitCode {
    eprintln!("scenario error: {err}");
    log::error!("scenario error: {err}");
    ExitCode::from(EXIT_SCENARIO_ERROR)
}

struct DataDir {
    path: PathBuf,
    temporary: bool,
}

fn prepare_data_dir(data_dir: Option<&Path>) -> Result<DataDir, ScenarioError> {
    let data_dir = match data_dir {
        Some(path) => DataDir {
            path: path.to_path_buf(),
            temporary: false,
        },
        None => {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default();
            DataDir {
                path: std::env::temp_dir().join(format!(
                    "kashikishi-e2e-{}-{}",
                    std::process::id(),
                    nanos
                )),
                temporary: true,
            }
        }
    };
    std::fs::create_dir_all(&data_dir.path).map_err(|source| ScenarioError::Directory {
        path: data_dir.path.clone(),
        source,
    })?;
    // 設定やメモのパスを解決する前に差し替える必要がある
    let _ = crate::paths::set_data_dir(data_dir.path.clone());
    Ok(data_dir)
}

fn load_scenario(path: &Path) -> Result<Vec<Action>, ScenarioError> {
    let mut actions = load_action_records(path)?;
    for action in actions.iter() {
        if let Some(name) = as_snapshot_name(action)
            && !is_valid_snapshot_name(name)
        {
            return Err(ScenarioError::InvalidSnapshotName {
                name: name.to_string(),
            });
        }
    }
    let exit = Action::new_command("system", "exit");
    if actions.last() != Some(&exit) {
        actions.push(exit);
    }
    Ok(actions)
}

fn is_valid_snapshot_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn load_config(path: Option<&Path>) -> Result<KashikishiConfig, ScenarioError> {
    let Some(path) = path else {
        return Ok(KashikishiConfig::default());
    };
    let to_error = |message: String| ScenarioError::Config {
        path: path.to_path_buf(),
        message,
    };
    let json = std::fs::read_to_string(path).map_err(|err| to_error(err.to_string()))?;
    serde_json::from_str(&json).map_err(|err| to_error(err.to_string()))
}

fn snapshot_dir(test_args: &TestArgs) -> PathBuf {
    test_args.snapshot_dir.clone().unwrap_or_else(|| {
        let name = test_args
            .scenario
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "scenario".to_string());
        PathBuf::from("target").join("e2e").join(name)
    })
}

enum ScenarioOutcome {
    Headless(HeadlessExit),
    /// ウィンドウを閉じるか、シナリオの最後の `system:exit` で終了した
    Headed,
}

fn run_scenario(test_args: &TestArgs) -> Result<ScenarioOutcome, ScenarioError> {
    let actions = load_scenario(&test_args.scenario)?;
    let config = load_config(test_args.config.as_deref())?;

    // 環境によって字形が変わらないように埋め込みフォントだけを使う
    let mut font_repository = FontRepository::new(FontCollector::default());
    font_repository.add_fallback_font_from_binary(FONT_DATA.to_vec(), None);
    font_repository.add_fallback_font_from_binary(EMOJI_FONT_DATA.to_vec(), None);

    // どちらの時計も 0 から始まるので、記録の基準時刻も 0 にする
    font_rasterizer::time::set_clock_mode(if test_args.headed {
        font_rasterizer::time::ClockMode::StepByStep
    } else {
        font_rasterizer::time::ClockMode::Fixed
    });
    let mut action_recorder =
        ActionRecorder::new_with_time(Box::new(InMemoryActionRecordRepository::new(actions)), 0);
    action_recorder.start_replay();

    let shader_art = config
        .background_shader
        .as_deref()
        .and_then(font_rasterizer::builtin_shader_art::find_by_name)
        .map(|source| source.to_string());
    let color_theme = config.editor_settings.color_theme();
    let callback = KashikishiCallback::new(test_args.window_size, config, action_recorder);
    let support = SimpleStateSupport {
        window_icon: None,
        window_title: "Kashikishi".to_string(),
        window_size: test_args.window_size,
        callback: Box::new(callback),
        quarity: Quarity::Middle,
        color_theme,
        flags: if test_args.headed {
            Flags::DEFAULT
        } else {
            Flags::empty()
        },
        font_repository,
        performance_mode: false,
        background_image: None,
        shader_art,
    };

    if test_args.headed {
        // 目視確認用なので、スナップショットは撮らない（snapshot コマンドは何もしない）
        pollster::block_on(run_support(support));
        return Ok(ScenarioOutcome::Headed);
    }

    let snapshot_dir = snapshot_dir(test_args);
    // 前回の実行結果が混ざらないように出力先を空にする
    if snapshot_dir.exists() {
        std::fs::remove_dir_all(&snapshot_dir).map_err(|source| ScenarioError::Directory {
            path: snapshot_dir.clone(),
            source,
        })?;
    }
    std::fs::create_dir_all(&snapshot_dir).map_err(|source| ScenarioError::Directory {
        path: snapshot_dir.clone(),
        source,
    })?;

    let options = HeadlessOptions {
        frame_gain: Duration::from_millis(test_args.frame_ms as u64),
        max_frames: test_args.max_frames,
    };

    let mut save_error = None;
    let exit = pollster::block_on(run_headless(support, options, |snapshot| {
        if save_error.is_none()
            && let Err(err) = save_snapshot(&snapshot_dir, &snapshot)
        {
            save_error = Some(err);
        }
    }));
    match save_error {
        Some(err) => Err(err),
        None => Ok(ScenarioOutcome::Headless(exit)),
    }
}

fn save_snapshot(dir: &Path, snapshot: &Snapshot) -> Result<(), ScenarioError> {
    let image_path = dir.join(format!("{}.png", snapshot.name));
    snapshot
        .image
        .save(&image_path)
        .map_err(|err| ScenarioError::SaveSnapshot {
            path: image_path.clone(),
            message: err.to_string(),
        })?;
    if let Some(text) = &snapshot.text {
        let text_path = dir.join(format!("{}.txt", snapshot.name));
        std::fs::write(&text_path, text).map_err(|err| ScenarioError::SaveSnapshot {
            path: text_path.clone(),
            message: err.to_string(),
        })?;
    }
    log::info!(
        "snapshot saved: {} (frame {})",
        snapshot.name,
        snapshot.frame
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_window_size_accepts_width_x_height() {
        let size = parse_window_size("640x480").unwrap();
        assert_eq!((size.width, size.height), (640, 480));
        assert!(parse_window_size("640").is_err());
        assert!(parse_window_size("0x480").is_err());
    }

    #[test]
    fn snapshot_name_validation() {
        assert!(is_valid_snapshot_name("01_start-up"));
        assert!(!is_valid_snapshot_name(""));
        assert!(!is_valid_snapshot_name("../evil"));
        assert!(!is_valid_snapshot_name("名前"));
    }
}
