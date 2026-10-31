//! kashikishi の E2E テスト。
//!
//! `e2e/scenarios/*.jsonl` のシナリオを `kashikishi test` で再生し、
//! `e2e/snapshots/<シナリオ名>/` の期待値と比較する。
//! GPU を使うため通常の `cargo test` では実行せず、`mise r e2e` で実行する。
//! `UPDATE_SNAPSHOTS=1` を指定すると期待値を更新する。
//! `E2E_HEADED=1`（`mise r e2e-headed`）を指定すると、各シナリオをウィンドウで再生するだけで比較はしない。

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Command,
};

use image::{Rgba, RgbaImage};

/// 画素ごとのチャンネル差がこの値を超えたら差分のある画素とみなす
const CHANNEL_TOLERANCE: u8 = 8;
/// 差分のある画素の割合がこの値以下なら一致とみなす
const MAX_DIFF_PIXEL_RATIO: f64 = 0.005;

fn e2e_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("e2e")
}

fn scenarios() -> Vec<PathBuf> {
    let mut scenarios: Vec<PathBuf> = std::fs::read_dir(e2e_dir().join("scenarios"))
        .expect("read scenarios dir")
        .map(|entry| entry.expect("read scenario entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .collect();
    scenarios.sort();
    scenarios
}

fn file_names(dir: &Path) -> BTreeSet<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return BTreeSet::new();
    };
    entries
        .map(|entry| entry.expect("read snapshot entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext == "png" || ext == "txt")
        })
        .filter_map(|path| Some(path.file_name()?.to_string_lossy().to_string()))
        .filter(|name| !name.ends_with(".diff.png"))
        .collect()
}

fn run_scenario(scenario: &Path, snapshot_dir: &Path, headed: bool) -> Result<(), String> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kashikishi"));
    command.arg("test").arg("--scenario").arg(scenario);
    if headed {
        command.arg("--headed");
    } else {
        command.arg("--snapshot-dir").arg(snapshot_dir);
    }
    let output = command
        .output()
        .map_err(|err| format!("failed to run kashikishi: {err}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "kashikishi test failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn compare_text(expected: &Path, actual: &Path) -> Result<(), String> {
    // git の autocrlf で改行コードが変わることがあるので正規化して比較する
    let read = |path: &Path| {
        std::fs::read_to_string(path)
            .map(|s| s.replace("\r\n", "\n"))
            .map_err(|err| format!("failed to read {}: {err}", path.display()))
    };
    let (expected_text, actual_text) = (read(expected)?, read(actual)?);
    if expected_text == actual_text {
        Ok(())
    } else {
        Err(format!(
            "text mismatch: {}\n--- expected\n{expected_text}\n--- actual\n{actual_text}",
            actual.display()
        ))
    }
}

fn compare_image(expected: &Path, actual: &Path) -> Result<(), String> {
    let open = |path: &Path| {
        image::open(path)
            .map(|image| image.to_rgba8())
            .map_err(|err| format!("failed to open {}: {err}", path.display()))
    };
    let (expected_image, actual_image) = (open(expected)?, open(actual)?);
    if expected_image.dimensions() != actual_image.dimensions() {
        return Err(format!(
            "image size mismatch: {} expected {:?} but {:?}",
            actual.display(),
            expected_image.dimensions(),
            actual_image.dimensions()
        ));
    }

    let (width, height) = expected_image.dimensions();
    let mut diff_image = RgbaImage::new(width, height);
    let mut diff_pixels = 0u64;
    for (x, y, expected_pixel) in expected_image.enumerate_pixels() {
        let actual_pixel = actual_image.get_pixel(x, y);
        let differs = expected_pixel
            .0
            .iter()
            .zip(actual_pixel.0.iter())
            .any(|(e, a)| e.abs_diff(*a) > CHANNEL_TOLERANCE);
        if differs {
            diff_pixels += 1;
            diff_image.put_pixel(x, y, Rgba([255, 0, 0, 255]));
        } else {
            // 一致した画素は暗いグレーにして差分の位置を見やすくする
            let [r, g, b, _] = actual_pixel.0;
            let gray = ((r as u16 + g as u16 + b as u16) / 12) as u8;
            diff_image.put_pixel(x, y, Rgba([gray, gray, gray, 255]));
        }
    }

    let ratio = diff_pixels as f64 / (width as u64 * height as u64) as f64;
    if ratio <= MAX_DIFF_PIXEL_RATIO {
        return Ok(());
    }
    let diff_path = actual.with_extension("diff.png");
    let _ = diff_image.save(&diff_path);
    Err(format!(
        "image mismatch: {} ({diff_pixels} pixels, {:.3}% differ). diff: {}",
        actual.display(),
        ratio * 100.0,
        diff_path.display()
    ))
}

fn compare_snapshots(expected_dir: &Path, actual_dir: &Path) -> Vec<String> {
    let expected_names = file_names(expected_dir);
    let actual_names = file_names(actual_dir);
    let mut errors = Vec::new();
    for name in expected_names.difference(&actual_names) {
        errors.push(format!("snapshot not taken: {name}"));
    }
    for name in actual_names.difference(&expected_names) {
        errors.push(format!(
            "unexpected snapshot: {name} (run with UPDATE_SNAPSHOTS=1 to accept)"
        ));
    }
    for name in expected_names.intersection(&actual_names) {
        let (expected, actual) = (expected_dir.join(name), actual_dir.join(name));
        let result = if name.ends_with(".png") {
            compare_image(&expected, &actual)
        } else {
            compare_text(&expected, &actual)
        };
        if let Err(err) = result {
            errors.push(err);
        }
    }
    errors
}

fn update_snapshots(expected_dir: &Path, actual_dir: &Path) -> Result<(), String> {
    if expected_dir.exists() {
        std::fs::remove_dir_all(expected_dir).map_err(|err| err.to_string())?;
    }
    std::fs::create_dir_all(expected_dir).map_err(|err| err.to_string())?;
    for name in file_names(actual_dir) {
        std::fs::copy(actual_dir.join(&name), expected_dir.join(&name))
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

#[test]
#[ignore = "GPU が必要なため `mise r e2e` で実行する"]
fn e2e_scenarios() {
    let update = std::env::var("UPDATE_SNAPSHOTS").is_ok_and(|v| v == "1");
    let headed = std::env::var("E2E_HEADED").is_ok_and(|v| v == "1");
    let actual_root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("e2e");
    let scenarios = scenarios();
    assert!(!scenarios.is_empty(), "no scenario found");

    let mut failures = Vec::new();
    for scenario in scenarios {
        let name = scenario
            .file_stem()
            .expect("scenario file name")
            .to_string_lossy()
            .to_string();
        let actual_dir = actual_root.join(&name);
        let expected_dir = e2e_dir().join("snapshots").join(&name);

        if headed {
            println!("replaying {name} in a window");
        }
        if let Err(err) = run_scenario(&scenario, &actual_dir, headed) {
            failures.push(format!("[{name}] {err}"));
            continue;
        }
        // ウィンドウでの再生は目視確認用なので、スナップショットを撮らず比較もしない
        if headed {
            continue;
        }
        if update {
            if let Err(err) = update_snapshots(&expected_dir, &actual_dir) {
                failures.push(format!("[{name}] failed to update snapshots: {err}"));
            }
            continue;
        }
        failures.extend(
            compare_snapshots(&expected_dir, &actual_dir)
                .into_iter()
                .map(|err| format!("[{name}] {err}")),
        );
    }

    assert!(
        failures.is_empty(),
        "e2e scenarios failed:\n{}",
        failures.join("\n")
    );
}
