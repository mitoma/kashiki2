---
title: E2E Performance
kind: decision
status: draft
updated: 2026-11-01
source_refs:
  - ../../../mise.toml
  - ../../../kashikishi/tests/e2e_scenarios.rs
  - ../../../kashikishi/src/scenario_test.rs
  - ../../../ui_support/src/headless.rs
  - ../../../ui_support/src/render_state.rs
related_pages:
  - ../concepts/world-scene-architecture.md
  - ../components/shader-art-system.md
---

# E2E Performance

## 決定

E2E（`mise r e2e`）の高速化は、現時点では見送る（保留）。
デバッグビルドで 1 シナリオあたり約 20 秒かかるが、無理に対応するほどではないと判断した。
以下は再検討するときのための調査メモである。

## 前提: ヘッドレス実行の時計

- `kashikishi test` のヘッドレス実行は `ClockMode::Fixed` で動き、1 フレームごとに `--frame-ms`（既定 16ms）だけ内部時計を進める
- シナリオの `wait` は内部時計の差で判定するので、実時間では待っていない
- そのため「wait の間だけ時計を進める」案は、すでに実現されている

## 計測結果

`tagbase_memo` シナリオ（1101 フレーム、スナップショット 12 枚）を、`kashikishi.exe test` を直接起動して計測した。

| 構成 | 時間 | 備考 |
|---|---|---|
| debug（`--frame-ms` 16 / 100 / 500） | 17.4 / 12.6 / 10.1 秒 | フレーム数を減らしても約 10 秒は残る |
| debug、起動のみ（`--max-frames 1`） | 約 7.5〜7.9 秒 | |
| release | 約 28〜30 秒 | debug より遅い |
| release、起動のみ（`--max-frames 1`） | 約 22〜30 秒 | 2 回目も遅く、初回だけの遅れではない |
| release + `WGPU_DEBUG=1` | 約 10〜11 秒 | スナップショットはすべて期待値と一致 |

## release が遅い原因

- `RUST_LOG=trace` のログを見ると、起動時に DX12 で windfoil シェーダー（`vs_windfoil` / `fs_windfoil_non_zero`）を生成するところで、合計約 28 秒止まっていた
- wgpu の `InstanceFlags::from_build_config()` は、`debug_assertions` が有効なときに `DEBUG` フラグを立てる。DX12 の FXC はこのフラグがあると最適化を省く
- release ビルドでは最適化が有効になるため、FXC のコンパイルが極端に遅くなる
- フレーム処理そのものは release のほうが速い（release 約 3 秒、debug 約 10 秒）
- `render_state.rs` は `InstanceDescriptor::new_without_display_handle_from_env()` を使っているので、環境変数 `WGPU_DEBUG=1` で `DEBUG` フラグを立てられる
- DX12 を強制しているのはヘッドレス描画のときだけである。ウィンドウ表示での起動時間に影響があるかは確認していない

## 再検討するときの候補

1. release + `WGPU_DEBUG=1` で実行する（約 2 倍速くなる見込み）
   - `mise.toml` の `e2e` / `e2e-update` を `cargo test --release` に変える
   - `e2e_scenarios.rs` から子プロセスに `WGPU_DEBUG=1` を渡す。またはヘッドレス描画時にコード側で `InstanceFlags::DEBUG` を立てる
   - release ビルドの分、ビルド時間とディスク使用量が増える
2. DXC を使う（`WGPU_DX12_COMPILER=dxc`）。`dxcompiler.dll` を用意する必要があるため未検証
3. シナリオを並列に実行する
4. `[profile.dev.package."*"]` で依存ライブラリだけ最適化する。過去にビルドが遅くなるため見送った経緯がある（`Cargo.toml` のコメント）
5. スナップショットを撮るときだけ GPU から読み出す

## 注意

- `kashikishi.exe` は release では `windows_subsystem = "windows"` になるため、PowerShell から `& exe ...` で起動すると終了を待たずに戻る。計測では `Start-Process -Wait` を使う
- ログは標準出力ではなく `%LOCALAPPDATA%\kashikishi\logs\kashikishi.log` に出力される
