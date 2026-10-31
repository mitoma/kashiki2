# シナリオ再生による kashikishi の E2E テストモード 設計

## 概要

kashikishi の起動直後から、操作（`Action`）の列を順番に流して一連の挙動を確かめ、決めたタイミングでスナップショット（画像とテキスト）を保存する **シナリオ再生モード** を追加します。

既存の `action-recorder` の replay 機能と、`ui_support::generate_images` が持つヘッドレス描画（ウィンドウなしで画像に描画する機能）を組み合わせます。`test` サブコマンドでシナリオを渡すと、ウィンドウを開かずに決定的なタイミングで再生し、終わったら自動で終了します。テスト用のオプションはすべて `test` サブコマンドの下に置き、通常起動のオプション一覧（`kashikishi --help`）には出てこないようにします。
得られたスナップショットを期待値と比べる E2E テストのハーネスも、同じ仕組みの上に作ります。

```powershell
kashikishi test --scenario kashikishi\e2e\scenarios\open_memo.jsonl --snapshot-dir target\e2e\open_memo
```

## 目的

- 起動 → モード切替 → 入力 → モーダル操作といった **アプリ全体を通した挙動** の回帰を検出する
- 人が操作して記録したもの、または手で書いたシナリオを、何度でも同じ結果になるように再生する
- 結果を画像とテキストで残し、レビューや差分比較に使えるようにする

## 現状調査

### action-recorder（`ui_support/src/action_recorder.rs`）

- `ActionRecorder` には `Record` / `Replay` / `None` の 3 つのモードがある
- 記録するときは、各 Action の前に `Command("action_recorder", "wait", Integer(経過ms))` を挟む
- 再生は `replay(&UiContext)` を毎フレーム呼んで進める。`wait` が来たら `now_millis()` の経過時間で待ち、それ以外の Action は `context.register_action` でキューに積む
- 保存先は `ActionRecordRepository` トレイトで抽象化されている
  - `FileActionRecordRepository` は固定パス `record.jsonl`（カレントディレクトリ）に JSON Lines で読み書きし、失敗すると `unwrap` で panic する
  - kashikishi は `InMemoryActionRecordRepository` だけを使っているため、録画はプロセスの外に出せない
- 気付いた点
  - 特殊コマンド `wait` の名前空間は `action_recorder`（アンダースコア）だが、`NamespaceActionProcessors` の名前空間は `action-recorder`（ハイフン）で、表記がそろっていない
  - `ReplayMode::Fast` は経過時間を `/10` しているので、実際には再生が **10 倍遅く** なる（本計画では使わない）
  - **記録が `KashikishiCallback::action()` の中で行われている**。このため、ユーザー入力が元になった Action だけでなく、SelectBox などが `register_action` で派生的に発行した Action（例: `mode:category`）も記録される。再生するとユーザー入力（Enter）と派生 Action の両方が流れ、**二重に実行される**

### ヘッドレス描画（`ui_support/src/lib.rs`）

- `generate_images` / `generate_image_iter` は `RenderTargetRequest::Image` でオフスクリーンに描画し、フレームごとに `ImageBuffer<Rgba<u8>>` を返す
  - `ClockMode::Fixed` にして、1 フレームごとに `increment_fixed_clock(frame_gain)` で時計を進める。このためアニメーションと `wait` が決定的になる
  - Windows では Image ターゲットのとき DX12 を強制する（`render_state.rs`）
  - 1 フレームの処理順は `action_queue` の消化 → `update`（ここで replay が Action を積む）→ `render` → 時計を進める → `post_action_queue` の消化
  - `InputResult::SendExit` は `handle_action_result` が `Some` で返すが、呼び出し側が捨てているので **終了できない**
- `sample_codes/apng_gen` は `ActionRecorder::new_with_time(repo, 0)` と `start_replay()`、`generate_images` を組み合わせて APNG を作っている。今回の構成の前例になる

### kashikishi 本体（`kashikishi/src/main.rs`）

- `Args`（clap）には `--performance-mode` / `--use-embedded-font` / `--ime-on-the-fly` / `--font-names` / `--clear-glyph-cache` がある
- `run()` で `SimpleStateSupport` を組み立て、`run_support`（winit のウィンドウ）で起動する。時計は `StepByStep`
- `KashikishiCallback::init` で `StartWorld` を作り、`world:fit-by-direction` を post action に積む

### E2E を不安定にする要因と副作用

| 種類 | 箇所 | 影響 |
|---|---|---|
| 設定の読み書き | `KashikishiConfig::load/save`（`~/.config/kashikishi/config.json`） | ユーザー環境に依存する。終了時に上書きされる |
| メモの読み書き | `CategorizedMemos::load_memos/save_memos`（`~/.config/kashikishi/categorized_memos.json`） | ユーザーのメモを読み、テスト中の入力で上書きしてしまう |
| システムフォント | `FontCollector::add_system_fonts`、`config.font` | 環境ごとに字形が変わる |
| 現在時刻 | `insert_date_select`（`chrono::Local::now`）、メモのバックアップ名 | 表示内容が実行日時で変わる |
| クリップボード | `world:copy-display`（arboard） | OS のクリップボードを書き換える |
| GPU / ドライバ | 描画全般 | 環境によって画素がわずかに変わる |
| アニメーション | イージング、カメラ移動 | 撮影するフレームによって結果が変わる |

## 要件

1. `test` サブコマンドでシナリオファイルを渡すと、ウィンドウを開かずに起動直後から再生する。テスト用オプションは通常起動のオプションと混ざらないようにする
2. シナリオの中で、任意のタイミングで名前付きのスナップショットを要求できる
3. 同じシナリオ・同じ環境なら、毎回同じ結果になる（時計・フォント・データを固定する）
4. ユーザーの設定やメモを読まず、書き換えもしない
5. 再生が終わったら自動で終了し、成功と失敗を終了コードで返す
6. 既存の録画機能で作ったファイルを、そのままシナリオとして使える
7. ネイティブ専用とする（wasm32 では無効）

## 全体設計

```text
kashikishi test --scenario s.jsonl --snapshot-dir out/
  │
  ├─ Args 解析 → Command::Test(TestArgs)
  ├─ 決定的な環境を作る
  │    ├─ データディレクトリ: 一時ディレクトリ（または --data-dir）
  │    ├─ 設定: デフォルト設定（または --config で指定したもの）
  │    └─ フォント: 埋め込みフォントのみ
  ├─ KashikishiCallback::new(.., ScenarioReplay(FileActionRecordRepository(s.jsonl)))
  │    └─ ActionRecorder::new_with_time(repo, 0) → start_replay()
  └─ ui_support::run_headless(support, HeadlessOptions)
        loop (frame) {
          action_queue を消化
            ├─ action_recorder:snapshot(name) → ランナーが横取りして pending に積む
            ├─ SendExit → ループを抜ける
            └─ それ以外 → state.action()
          state.update()   … ActionRecorder::replay が次の Action を積む
          image = state.render()
          pending があれば out/<name>.png と out/<name>.txt を保存する
          時計を frame_ms だけ進める / post_action を消化
          max_frames を超えたらタイムアウトとして失敗
        }
        state.shutdown()   … 設定・メモは一時ディレクトリに保存される
```

### 1. コマンドライン（`kashikishi/src/main.rs` の `Args`）

clap のサブコマンドで、通常起動とテスト用の起動を分けます。サブコマンドを付けなければ、これまでどおりウィンドウで起動します。

```text
kashikishi [通常起動のオプション]                 # 従来どおり（ウィンドウ）
kashikishi [通常起動のオプション] record <PATH>   # 操作を記録してファイルに保存する（ウィンドウ）
kashikishi test --scenario <PATH> [テスト用のオプション]   # シナリオ再生（ヘッドレス）
```

```rust
#[derive(Parser, Debug, Clone)]
pub struct Args {
    // 既存の通常起動用オプション（performance_mode, use_embedded_font, ...）はそのまま
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// シナリオを再生してスナップショットを取る（E2E テスト用）
    Test(TestArgs),
    /// 操作を記録してファイルに保存する
    Record(RecordArgs),
}
```

- 通常起動のオプションには `global = true` を付けない。このため `kashikishi --help` にはテスト用オプションが出ず、`kashikishi test --help` には通常起動のオプションが出ない
- `test` は環境を固定するため、通常起動のオプションを使わない。clap の仕様上 `kashikishi --font-names X test ...` のようにサブコマンドの前に書くことはできてしまうので、その場合は値を無視し、警告をログに出す
- `record` はウィンドウで普通に操作するモードなので、通常起動のオプションをそのまま使う

#### `test` サブコマンドのオプション

| オプション | 既定値 | 内容 |
|---|---|---|
| `--scenario <PATH>` | 必須 | 再生するシナリオファイル |
| `--snapshot-dir <DIR>` | `target/e2e/<シナリオ名>` | スナップショットの出力先 |
| `--data-dir <DIR>` | 一時ディレクトリ | 設定やメモの保存先を差し替える |
| `--config <PATH>` | なし（デフォルト設定） | 初期設定（JSON） |
| `--window-size <WxH>` | `800x600` | 描画サイズ |
| `--frame-ms <MS>` | `16` | 1 フレームで固定時計を進める量 |
| `--max-frames <N>` | `36000`（約 10 分） | 無限ループを防ぐための上限。超えたら失敗 |
| `--headed` | なし | ウィンドウで実時間で再生する目視確認用のモード。スナップショットは撮らず、`--snapshot-dir` / `--frame-ms` / `--max-frames` は無視する |

#### `record` サブコマンドの引数

| 引数 | 内容 |
|---|---|
| `<PATH>` | `action-recorder:stop-record` を実行したときに、記録をこのファイルへ JSON Lines で保存する |

- `test` では `use_embedded_font` を強制し、`config.font` / `ascii_override_font` / `background_shader` を無視する（`--config` で明示されたものは除く）
- `--data-dir` を指定しない場合は、`std::env::temp_dir()` の下に実行ごとのディレクトリを作り、終わったら削除する。こうすることで、毎回 **まっさらな起動直後** の状態から始まる
- 既存のメモがある状態から始めたいときは、`--data-dir` にフィクスチャをコピーしたディレクトリを渡す（ハーネス側でコピーする）
- 終了コードは次のとおり
  - `0`: 正常終了
  - `1`: シナリオの読み込みエラー
  - `2`: タイムアウト（`max-frames` 超過）
  - `3`: panic
- リリースビルドは `windows_subsystem = "windows"` のため標準出力が見えない。このため、結果は終了コードと出力ファイルで判断する

### 2. シナリオフォーマット

既存の録画と互換にするため、**`stroke_parser::Action` を serde でシリアライズした JSON Lines** を正式なフォーマットとします。

```jsonl
{"Command":["action_recorder","wait",{"Integer":1000}]}
{"Command":["action_recorder","snapshot",{"String":"01_start"}]}
{"Command":["edit","return","None"]}
{"Command":["action_recorder","wait",{"Integer":1500}]}
{"Command":["action_recorder","snapshot",{"String":"02_memo_opened"}]}
{"ImeInput":"こんにちは炊紙"}
{"Command":["action_recorder","wait",{"Integer":1000}]}
{"Command":["action_recorder","snapshot",{"String":"03_typed"}]}
{"Command":["system","exit","None"]}
```

特殊コマンドは、既存の `wait` に合わせて名前空間 `action_recorder` にまとめます。

| コマンド | 引数 | 意味 |
|---|---|---|
| `action_recorder:wait` | `Integer(ms)` | 既存のもの。固定時計で `ms` だけ待つ（`ceil(ms / frame_ms)` フレーム） |
| `action_recorder:snapshot` | `String(name)` | 新規。スナップショットを要求する。`name` はファイル名に使う（`[A-Za-z0-9_-]` のみ許可） |

- シナリオの最後に `system:exit` がなければ、ローダーが自動で付け足す
- 名前空間の表記ゆれ（`action_recorder` と `action-recorder`）は、既存の録画ファイルとの互換を優先して今回は直さない。特殊コマンドの名前は定数にして、ソースの 1 か所で管理する
- 手で書きやすい DSL（`apng_gen` の `<enter>` や `<wait-1000>` のような記法、または TOML）は将来の拡張とする（後述）

### 3. スナップショットのタイミング

- `replay()` は `update` の中で呼ばれ、Action を `action_queue` に積むだけである。積まれた Action は **次のフレームの頭** でまとめて処理される
- `snapshot` も同じキューに Action として積む。ランナーはキューを消化するときに `snapshot` を横取りし、そのフレームを描画したあとに保存する
  - これで、`snapshot` より前に積まれた Action は必ず処理済みになる（順序が保証される）
  - post action（例: モード切替後の `world:fit-by-direction`）とイージングは描画後に反映される。このため、**スナップショットの前に `wait` を入れて落ち着かせる** ことをシナリオの書き方の決まりとする
- 横取りはランナー（`run_headless`）で行う。ウィンドウモードで `snapshot` が流れた場合は、どのプロセッサにも当たらず Noop になる（害はない）

### 4. スナップショットの中身

| ファイル | 内容 | 用途 |
|---|---|---|
| `<name>.png` | 描画結果（`ImageBuffer<Rgba<u8>>`） | 見た目の回帰。許容誤差をつけて比べる |
| `<name>.txt` | アプリの論理状態をテキストにしたもの | 環境に依存しない厳密な比較 |

テキストスナップショットを得るために、`SimpleStateCallback` に既定実装つきのメソッドを追加します。

```rust
pub trait SimpleStateCallback {
    // ...
    /// E2E テスト用に、現在の論理状態をテキストで返す
    fn snapshot_text(&self) -> Option<String> {
        None
    }
}
```

kashikishi では、次の内容を出力します（安定した順序のテキスト）。

- 現在の `ModalWorld` の種類（start / category / presentation / help）
- レイアウト、方向、フォーカスしているモデルの番号
- `World::debug_snapshot()` によるモデルとモーダルのツリー。ノードごとに種類と主な状態を出す（`TextEdit` は方向とテキスト、`SelectBox` は選択位置・絞り込み後の件数・全件数、`StackLayout` はフォーカス位置など）
- 浮動小数点の値（位置や拡大率）は環境やアニメーションの途中で揺れやすいので出さない

実装は `kashikishi/src/snapshot_text.rs` にあります。

GPU の差に左右されない比較はテキストで行い、画像はおおまかな見た目の確認に使う、という役割分担にします。

### 5. ui_support 側の変更

- `action_recorder.rs`
  - `FileActionRecordRepository` が任意のパスを受け取れるようにする（`new(path)`）。`load` は `Result` を返す新しい関数を用意し、パースエラーの行番号を報告する。エラー型は `thiserror` で定義する
  - 特殊コマンドの定数（`RECORDER_NAMESPACE = "action_recorder"`、`WAIT`、`SNAPSHOT`）を公開する
  - `is_replaying()` を追加する（ランナーの終了判定の補助に使う）
- `headless.rs`（新しいモジュール）
  - `run_headless(support, HeadlessOptions, on_snapshot: impl FnMut(Snapshot)) -> HeadlessExit` を追加する
    - `HeadlessOptions { frame_gain, max_frames }`、`Snapshot { name, frame, image, text }`、`HeadlessExit::{Exited, Timeout}`
    - `SendExit` を受け取ったら `shutdown` してループを抜ける（`generate_images` では捨てていた部分）
  - `generate_images` / `generate_image_iter` との共通化は行わず、別の関数として実装した。`generate_images` は呼び出し側がフレームを引き出すイテレーター型で、終了やスナップショットの扱いが異なるため、共通化すると既存の動作を変える範囲が大きくなる。重複が問題になったら 1 フレーム分の処理を切り出す
- `SimpleStateCallback::snapshot_text` を追加する（既定値は `None` なので、既存の実装は変更不要）

### 6. kashikishi 側の変更

- 新しく `paths` モジュールを作る。`OnceLock<PathBuf>` にデータディレクトリを持たせ、`KashikishiConfig::config_file` と `categorized_memos::memos_file` はここを参照する。既定値は従来どおり `~/.config/kashikishi`
  - あわせて、`config.json` を作る前に `create_dir_all` するようにする
- `KashikishiCallback::new` が `ActionRecorder` を外から受け取れるようにする
  - 通常: `InMemoryActionRecordRepository`（現状どおり）
  - シナリオ: `load_action_records(path)` で読み込み（不正な行はここでエラーにする）、`ActionRecorder::new_with_time(Box::new(InMemoryActionRecordRepository::new(actions)), 0)` を作って `start_replay()` する
- `main()` で `args.command` によって分岐する（`test` は `#[cfg(not(target_arch = "wasm32"))]`）
  - `Some(Test)`: `scenario_test::run_test` を呼び、`ExitCode` を返す。`set_clock_mode(ClockMode::Fixed)` にして、`run_support` の代わりに `run_headless` を呼ぶ
  - `None`: 従来どおり `run()` から `run_support`
  - `Some(Record)`: 従来どおり `run()` から `run_support`。ただし `ActionRecorder` の保存先を `FileActionRecordRepository::new(path)` にする
- `snapshot_text` を実装する
- **録画位置の修正**: `record()` の呼び出しを `action()` から `input()` に移し、`winit_window_event_to_action` で得た **ユーザー入力が元の Action だけ** を記録する（IME イベントも `input()` を通るので記録される）。これで派生 Action の二重実行を防ぐ
- 録画をファイルに書き出せるようにする。`kashikishi record <PATH>` で起動したときは、`stop-record` で `FileActionRecordRepository` に保存する。これで「人が操作して記録し、それをシナリオにする」流れができる

### 7. E2E テストハーネス

```text
kashikishi/
  e2e/
    scenarios/
      open_memo.jsonl
      command_palette.jsonl
    fixtures/              # 必要なら --data-dir の初期データ
      with_memos/
    snapshots/             # 期待値（git 管理）
      open_memo/
        01_start.png
        01_start.txt
  tests/
    e2e_scenarios.rs
```

- `kashikishi/tests/e2e_scenarios.rs` で、`env!("CARGO_BIN_EXE_kashikishi")` を使って `kashikishi test --scenario ...` を起動し、出力を `snapshots/` と比べる
  - テキスト: 完全に一致することを確認する
  - 画像: 画素ごとの差が閾値（各チャンネル 8）を超える画素の割合が、許容率（0.5%）以下であることを確認する。失敗したら実行結果の隣に `<name>.diff.png` を出力する（差分のある画素を赤で示す）
  - 実行結果は `CARGO_TARGET_TMPDIR`（`target/tmp/e2e/<scenario>/`）に出力する
  - テキストは git の autocrlf の影響を受けないよう、改行コードを LF にそろえてから比べる
  - 環境変数 `UPDATE_SNAPSHOTS=1` を付けると、期待値を上書き更新する
- GPU が必要で時間もかかるため、テストには `#[ignore]` を付ける。通常の `cargo test --all` では実行しない
- `mise.toml` に次のタスクを追加する
  - `e2e`: `cargo test -p kashikishi --test e2e_scenarios -- --ignored`
  - `e2e-update`: 上と同じ処理を `UPDATE_SNAPSHOTS=1` 付きで実行する
  - `e2e-headed`: 上と同じ処理を `E2E_HEADED=1` 付きで実行する。各シナリオを `--headed` でウィンドウ再生するだけで、比較はしない
- CI（`.github/workflows/rust.yml`）への組み込みは、GPU がない環境の扱い（lavapipe / WARP）を確かめてから別途行う。画像の期待値はプラットフォームごとに差が出る可能性があるため、当面は **開発機（Windows）で画像とテキストを比べ、CI ではテキストだけを比べる** 方針を候補とする

## 実装ステップ

1. **ui_support: 録画機能の整備**
   - `FileActionRecordRepository::new(path)`、`Result` を返す読み込み、特殊コマンドの定数、`is_replaying()`
2. **kashikishi: 録画位置の修正と `record` サブコマンド**
   - `Args` にサブコマンド（`Command`）を導入する。`record()` を `input()` に移し、ファイルへの保存に対応する
3. **kashikishi: データディレクトリの差し替え**
   - `paths` モジュールを作り、設定とメモのパス解決を一本化する（`test --data-dir` の土台）
4. **ui_support: `run_headless` と `snapshot` の横取り、`snapshot_text`**
   - `SendExit` で終われるヘッドレスのループを `headless.rs` に用意する（`generate_images` とは共通化しない）
5. **kashikishi: `test` サブコマンド**
   - 決定的な環境（固定時計、埋め込みフォント、デフォルト設定、一時データディレクトリ）を作り、`run_headless` を呼ぶ。スナップショットを書き出し、終了コードを返す
6. **E2E ハーネスとサンプルシナリオ**
   - 「起動 → メモ帳を開く → 文字入力」と「コマンドパレットを開いて絞り込む」の 2 本を用意し、mise タスクを追加する
7. **ドキュメント**
   - `doc/project.md` などにシナリオの書き方と実行方法を追記する

各ステップは独立してコミットできる単位です。1〜3 はシナリオモードがなくても単独で価値があります。

## 検討事項

### 0. コマンドラインの構成

- 採用: `test` / `record` をサブコマンドにする。テストや記録にしか使わないオプションが、通常起動のヘルプに出てこない
- `record` は通常起動のオプションを使うので、`kashikishi [通常オプション] record <PATH>` の形にする。`kashikishi record` 側に通常起動のオプションを重複して定義することはしない
- 将来 `test` に機能が増えたら（シナリオの一覧表示や、期待値との比較まで本体で行うなど）、`test run` / `test list` のようにさらにサブコマンドを分けることを検討する

### 1. スナップショットの横取りをどこで行うか

- A案（採用）: ランナー（`run_headless`）がキューを消化するときに横取りする。`InputResult` や `ActionProcessor` に手を入れずに済み、ウィンドウモードの処理にも影響しない
- B案: `InputResult::RequestSnapshot(String)` を追加する。ウィンドウモードでも撮影できるが、`lib.rs` にある 3 か所の `match` をすべて直す必要がある
- → まず A案で進め、ウィンドウモードで撮りたくなったら B案に広げる

### 2. アニメーションが落ち着くのを待つ方法

- 当面はシナリオに `wait` を明示的に書く
- 将来は `action_recorder:wait-idle` のように「イージングとカメラが止まるまで待つ」コマンドが欲しい。ただし `UiContext` や World からアニメーション中かどうかを集めて判定する仕組みが要る
- `--config` で `char_easings_preset: ZeroMotion` を指定すると、文字のアニメーションはなくせる（カメラの動きは残る）

### 3. 時刻に依存する機能

- `insert_date_select` などは実行日時で表示が変わる。シナリオでは使わないことを決まりとする
- 必要になったら、時刻の取得を差し替え可能にする（シナリオモードでは固定日時にする）ことを別途検討する

### 4. クリップボード

- `world:copy-display` は OS のクリップボードを書き換える。シナリオモードでは何もしないようにするか、出力をスナップショットに含めるかを検討する（当面はシナリオで使わない）

### 5. 手書きしやすいシナリオ形式

- JSON Lines は録画と互換だが、手で書くには冗長である
- 候補: `apng_gen` の `ActionRecordConverter` のようなテキスト DSL、または TOML の配列。どちらも、読み込んだら `Vec<Action>` に変換して同じ再生経路に流す
- → まずは JSON Lines のみに対応し、使ってみて必要なら DSL を追加する

### 6. 画像比較の閾値と、プラットフォームごとの期待値

- 閾値と許容率はハーネスの定数にし、シナリオごとに上書きできるようにするか検討する
- プラットフォームによって差が大きい場合は、`snapshots/<platform>/` のように分ける

### 7. glyph キャッシュ

- `font_rasterizer` の `cache` feature によるグリフキャッシュは、出力結果には影響しない想定である。不安定さが出たら、シナリオモードではキャッシュを無効にするか、一時ディレクトリに向ける

## 実装の優先順位

1. **高**: 録画機能の整備と、録画位置の修正（ステップ 1, 2）
2. **高**: データディレクトリの差し替え（ステップ 3）
3. **高**: `run_headless` と `test` サブコマンド、PNG とテキストのスナップショット（ステップ 4, 5）
4. **中**: E2E ハーネス、サンプルシナリオ、mise タスク（ステップ 6）
5. **低**: `wait-idle`、手書き用の DSL、CI への組み込み、ウィンドウモードでの撮影
