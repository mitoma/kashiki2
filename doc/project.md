# プロジェクト構造

## 概要

炊紙（kashikishi）は三次元空間上でテキストを編集できるテキストエディタです。  
Rust & WebGPU(wgpu)で開発されており、従来のテキストエディタにはない滑らかな編集体験を提供することを目指しています。

## テストと性能調査

`mise r test` は Boxington (`mbx`) 経由で `cargo test --all` を実行します。
単体・結合テストと有効な doctest を含み、GPU が必要な ignored テストは別途実行します。
コードを変更した際は `mise r check` も実行してください。

```powershell
mise r test
mise exec -- mbx test --workspace --doc  # 有効な doctest のみ実行
mise exec -- mbx analyze         # 直近ビルドの未キャッシュ原因・クリティカルパス
mise exec -- mbx explain --last  # 再ビルドせず、直近のキャッシュミス・bypass を確認
mise exec -- mbx stats --json    # キャッシュ統計（累積値）
```

doctest は `stroke_parser` に1件、`tagbase_memo` の README に2件あり、すべて `no_run` です。
サンプルコードがコンパイルできることだけを検証し、実行はしません。
doctest がない13クレートでは、空の検出処理を省くため `[lib] doctest = false` を設定しています。
これらのクレートに doctest を追加するときは、その `Cargo.toml` の設定を削除または `true` に変更し、
コードブロックを `no_run` にしたうえで `mise exec -- mbx test -p <クレート名> --doc` で確認してください。

`stats` はテスト関数の実行時間ではなく、コンパイルキャッシュの統計です。
変更前後の差分で読み、`analyze` と `explain --last` は調査したいビルドの直後に実行します。
Cargo の `Finished ... in ...` はビルド部分だけで、テスト・doctest やラッパーの起動時間は含みません。
テストハーネスの `finished in ...` にも doctest 自体のコンパイル時間は含まれません。

PowerShell で全体時間を測る場合は、初回のビルドと変更なしの再実行を分けます。
ログの行ごとに時刻を付けたり `Tee-Object` に通したりすると出力処理の時間も増えるため、
同じ出力方法で複数回比較してください。

```powershell
Measure-Command { mise r test *> $env:TEMP\kashikishi-test.log }
# ビルドだけを測る（テストと doctest は実行しない）
Measure-Command { mise exec -- mbx test --all --no-run *> $env:TEMP\kashikishi-test-build.log }
# Boxington を明示的に呼ばない場合との比較
Measure-Command { mise exec -- cargo test --all --no-run *> $env:TEMP\kashikishi-test-cargo.log }
```

### 2026-11-14 の Windows での調査

- 変更なしの `mise r test` は修正前が約 18.7〜19.0 秒、修正後が約 12.9〜13.5 秒。
  計測時のログ出力方法・マシン負荷に依存する値で、クリーンビルドの時間ではありません。
- 文字幅テストがプロセス全体のロガーを強制的に DEBUG にし、並列実行中の他のテストからも
  1 万行以上を出力していました。`is_test(true)` によるキャプチャと通常のログレベルに変更しました。
  `RUST_LOG=debug` でも成功したテストのログはキャプチャされ、失敗時の診断出力は維持します。
- 時計テストの実時間での 1 秒待機を廃止しました。グローバル時計を変更せず個別の時計を使い、
  System / Fixed / StepByStep の振る舞いを assertion で検証します。
- ウォームビルドは約 1.5〜1.8 秒。`--no-run` の比較では、Boxington 経由にすると
  起動などで約 3.4 秒増えました。ただし再ビルド時のキャッシュ効果とトレードオフなので、
  既定のタスクからは外していません。
- `mbx analyze` は通常の再実行で `rustdoc` をキャッシュ対象外と報告しました。
  調査時の `0 hits, 0 misses, 15 bypassed` はキャッシュ破損ではなく、doctest 処理によるものです。
- 追加調査では、3件の実行対象が2クレートに存在し、残る13クレートには doctest がありませんでした。
  空の13クレートでのみ doctest を無効にした結果、ウォーム状態の `mise r test` は
  変更前の約 14.9 秒から変更後の約 10.7〜12.1 秒になり、`rustdoc` の bypass は15から2になりました。
  時間はマシン負荷に依存します。また、設定変更直後はクレートの再ビルドが発生します。

## アーキテクチャ

このプロジェクトはRustのワークスペース構成を採用し、機能ごとに独立したクレートで構成されています。以下に主要なクレートとその役割を示します。

## 主要クレート

### kashikishi（メインアプリケーション）
- **パス**: `kashikishi/`
- **役割**: メインの実行ファイルとなるアプリケーション
- **依存関係**: 他の全てのクレートを統合し、最終的なテキストエディタを提供
- **メモモード**: 開始画面から従来の `CategorizedMemosWorld` と `TagbaseMemosWorld` を選択可能。タグメモは `~/kashikishi-docs` を保管庫として使用する。

#### 操作の記録と E2E テスト

`kashikishi record <PATH>` で起動すると、操作（`stroke_parser::Action`）を記録できます。
`action-recorder:stop-record` を実行すると、記録を `<PATH>` に JSON Lines で保存します。

`kashikishi test --scenario <PATH>` は、シナリオ（同じ JSON Lines 形式）をウィンドウなしで再生します。
固定時計、埋め込みフォント、デフォルト設定、一時データディレクトリを使うので、起動直後の状態から毎回同じ結果になります。
シナリオ中の `action_recorder:snapshot` の時点で、`<名前>.png`（描画結果）と `<名前>.txt`（ワールドの論理状態）を `--snapshot-dir`（既定は `target/e2e/<シナリオ名>`）に保存します。
終了コードは `0` が成功、`1` がシナリオのエラー、`2` がタイムアウト、`3` が panic です。
`--headed` を付けると、シナリオをウィンドウで実時間で再生します。シナリオを書くときの目視確認用で、スナップショットは撮りません（`--snapshot-dir`、`--frame-ms`、`--max-frames` は無視します）。
その他のオプションは `kashikishi test --help` を参照してください。

```jsonl
{"Command":["action_recorder","wait",{"Integer":1500}]}
{"Command":["action_recorder","snapshot",{"String":"01_start"}]}
{"Command":["edit","return","None"]}
{"ImeInput":"こんにちは"}
{"Command":["system","exit","None"]}
```

- `action_recorder:wait` は固定時計で指定したミリ秒だけ待ちます。アニメーションが落ち着くよう、スナップショットの前に入れます
- シナリオの末尾に `system:exit` がなければ自動で追加します
- 日付の挿入やクリップボードなど、環境に依存する操作はシナリオで使わないでください

E2E テストは `kashikishi/e2e/scenarios/*.jsonl` を再生し、`kashikishi/e2e/snapshots/<シナリオ名>/` の期待値と比べます。
テキストは完全一致、画像は許容誤差付きで比べ、不一致の画像は `target/tmp/e2e/` に `*.diff.png` を出力します。
GPU が必要なため通常の `cargo test` では実行されません。

```powershell
mise r e2e         # 期待値と比較する
mise r e2e-headed  # 各シナリオをウィンドウで再生して目で確認する（比較はしない）
mise r e2e-update  # 期待値を更新する（差分を目で確認してからコミットする）
mise r e2e-select         # 実行するシナリオをターミナルで選んでから期待値と比較する
mise r e2e-headed-select  # 選んだシナリオをウィンドウで再生する
mise r e2e-update-select  # 選んだシナリオの期待値だけを更新する
```

紹介動画用のシナリオ `kashikishi/demo/introduction.jsonl` もあります。E2E の対象ではなく、スナップショットも持ちません。
解説文をメモに入力しながら主な機能を順に実行します（約 2.5 分）。

```powershell
mise r demo        # 紹介シナリオをウィンドウで再生する
```

### font_rasterizer（フォントレンダリング）
- **パス**: `font_rasterizer/`
- **役割**: フォントのラスタライゼーションとレンダリング処理
- **特徴**: WebGPU(wgpu)を使用したGPUベースのフォント描画
- **ライブラリタイプ**: `cdylib`、`rlib`（WebAssemblyにも対応）

#### 描画方式

描画は **Windfoil** に統一しています。有向二次ベジェ曲線を storage buffer に格納し、
水平バンドと解析的な曲線積分で被覆率を計算します。距離依存の変形には曲線の事前投影を使用します。
通常画面とモーダルは同じ render pass と resolve で合成します。
通常モデルを奥から手前へ描画した後、モーダルを追加順に重ねます。
モーダルの Surface はテーマ背景色・不透明度 90% とし、専用テクスチャや背景ぼかしは使用しません。
閉じたモーダルは操作対象からすぐに外れ、背景・枠・文字が元の重なり順のまま 500ms でフェードアウトします。
`StackLayout` の Surface は枠線と同様に子要素間の余白も含むレイアウト全体を一枚で覆います。
Surface の設定は子要素へ伝播せず、子要素が個別に持つ Surface は維持します。
Surface は枠線の線幅の内縁（bound の 17/18 セル外側）まで広げます。
Border が `None` / `Rounded` のときは角丸枠線と同じ半径 4/9 セルの角丸、`Square` のときは四角形にします。
`StackLayout` は Surface があれば枠線がなくても bound に外側 1 セル分を含め、カメラが背景面まで収めます。

Legacy の描画実装・専用シェーダー・頂点/インデックス GPU バッファと、
描画方式の切り替え UI・アクション・`RenderingBackend`・`new_with_backend` 系 API は廃止しました。
既存設定の `rendering_backend`（`Legacy` / `Windfoil`）は読み込み時に無視され、
他の設定を維持したまま Windfoil を使用します。再保存時にこの項目は出力しません。

`RasterizerPipeline::new` と `RasterizerRenderrer::new` は常に Windfoil を使用します。
PNG 出力には `render_vector_vertex_to_png` またはその非同期版を使用します。
字形と SVG の CPU 側の三角形データおよび字形キャッシュ形式は維持し、輪郭辺から曲線を復元します。
GPU 曲線バッファは字形・SVG の登録時に生成し、失敗は登録元へ返します。
WebAssembly でも storage buffer を使用できる WebGPU の default limits を要求します。

GPU 回帰テストは通常のテストでは無効化されているため、必要に応じて次を実行します。

```powershell
cargo test -p font_rasterizer windfoil_ --lib -- --include-ignored
cargo test -p font_rasterizer --features cache cached_legacy_geometry_preserves_windfoil_curves --lib
```

### text_buffer（テキスト管理）
- **パス**: `text_buffer/`
- **役割**: テキストデータの格納と操作を担当
- **機能**: 
  - テキストバッファ管理
  - エディタ機能
  - カーソル（キャレット）制御
  - 文字種判定

### font_collector（フォント収集）
- **パス**: `font_collector/`
- **役割**: システムフォントの検索と収集
- **機能**: OS別フォントの自動検出とロード

### stroke_parser（入力解析）
- **パス**: `stroke_parser/`
- **役割**: キーボードとマウス入力の解析と処理
- **機能**: 
  - キー入力の処理
  - マウスアクションの処理
  - 入力ストロークの解析
  - `ActionStore::set_current_namespaces` による有効なコマンド名前空間と優先順位の指定。同一キーは名前空間の指定順、同一名前空間内は登録順で解決する
  - キーボード・マウス・ストローク継続判定に同じ名前空間条件を適用し、条件変更時は入力途中のストロークを破棄する。未設定時は従来互換、空リストはコマンド無効、`clear_current_namespaces` で制限解除

炊紙では `ModalWorld::command_namespace` がモード固有の名前空間を返し、起動時とモード切り替え時に共通名前空間より優先して有効化する。カテゴリーメモは `categorized-memo`、タグメモは `tagbase-memo` を使用し、保存などの共通操作は `kashikishi` に残す。

### ui_support（UI支援）
- **パス**: `ui_support/`
- **役割**: ユーザーインターフェースの支援機能
- **依存関係**: `nenobi`（外部ライブラリ）を使用

#### WorldLayout の 2 Pane

`WorldLayout::Split(Direction)` は、単一カメラの3D空間に2文書を配置します。
`Vertical` は左右分割、`Horizontal` は上下分割で、縦書き・横書きとは独立しています。
2文書のサイズと間隔を含めた領域を画面へフィットさせます。固定 viewport や文書の複製は行いません。
左右分割で `world:fit-width` / `world:fit-height` を選んでいるときは、フォーカス側の文書の高さを保ったまま、もう一方の文書を上下にずらして2文書のカーソルの高さを揃えます。
全体表示（`world:reset-zoom`、分割直後）では倍率が変わり続けないよう、2文書の中心の高さを揃えて並べます。

- `C-x 2` / `world:split-horizontal`: もう一方の文書を選択して上下分割
- `C-x 3` / `world:split-vertical`: もう一方の文書を選択して左右分割
- `C-x o` / `world:look-other-pane`: 配置を維持して Pane のフォーカスを切り替え
- `C-x b` / `world:switch-model`: 文書を選択し、フォーカス中の文書（分割中はフォーカス側 Pane の文書）と切り替え
- `C-x 1` / `world:unsplit`: 分割を解除し、現在の文書へフィット
- `world:change-layout-ui`: 直線・円・左右分割・上下分割を選択

選択画面は Enter で確定、`C-g` でキャンセルします。別文書が必要なので、2文書以上を開いてください。
分割中はフォーカスがある文書に `ThemeBackgroundHighlight` の Surface を付けて区別します。Border はユーザー設定を維持します。
フォーカス文書が元々 Surface を持つ場合は強調中だけ置き換え、フォーカス移動・分割解除・文書を閉じたときに元の設定へ戻します。
分割中も次／前の文書操作は全文書を巡回し、非表示の文書へ移動すると操作中の Pane の文書を置き換えます。
表示中のもう一方の文書へ移動した場合は、配置を維持してフォーカスだけを切り替えます。
`world:switch-model` でもう一方の Pane の文書を選んだ場合は、2つの Pane の文書を入れ替えます。
クリックの対象と通常描画は表示中の2文書に限定されます。
文書の編集・カーソル移動・サイズ変更・ウィンドウのリサイズ時は、分割中に直近で選んだフィット方法
（`world:reset-zoom` / `world:fit-width` / `world:fit-height`、分割直後は全体表示）で2文書全体へ再フィットします。
画面に収まらない方向はフォーカス中の文書のカーソルに追従します。
削除により文書が1つ以下になると直線レイアウトへ戻ります。
アクション再生では `world:split-horizontal` / `world:split-vertical` / `world:switch-model` に文書インデックス
（0始まりの文字列）を渡すと、選択画面を開かず実行できます。

### highlighter（シンタックスハイライト）
- **パス**: `highlighter/`
- **役割**: シンタックスハイライト機能
- **機能**:
  - Arborium（tree-sitter互換API）ベースの構文解析
  - 複数言語対応（Markdown、Rust、Java、Go、JSON、Bash）
  - 設定可能なハイライトカテゴリ定義
- **設定ファイル**: `asset/` ディレクトリ内のJSON形式定義ファイル
- **特徴**: 言語ごとの構文要素を柔軟にカテゴライズして色付け可能

### markdown_heading_splitter（Markdown見出し分割）
- **パス**: `markdown_heading_splitter/`
- **役割**: Markdown文書を見出し単位のセクションへ分割
- **機能**:
  - Arborium の Markdown grammar を用いた見出し抽出
  - ATX見出し（`#`）とSetext見出し（`===` / `---`）の両対応
  - 見出しレベル（H1〜H6）と本文ペアの返却
- **主な公開API**:
  - `split_headings(markdown: &str) -> Vec<(Heading, String)>`
  - `Heading::level()` / `Heading::title()`
- **利用箇所**: `kashikishi/src/world/markdown_presentation_world.rs` でMarkdownプレゼンテーション表示時の分割処理に利用
- **補足**: セクション本文は前後空白をトリムして返却され、見出しが存在しない文書は空配列を返す

### tagbase_memo（タグベースのメモ保管庫）
- **パス**: `tagbase_memo/`
- **役割**: Markdown と YAML front matter によるメモの永続化
- **機能**:
  - メモの作成・取得・更新・削除、作成日時順の一覧取得
  - タイトル・本文の検索、複数タグの AND 絞り込み、非表示タグ
  - 一時ファイルを使った保存、パス付きのエラー通知
- **補足**: GUI への組み込みと旧形式からの移行は未対応
- **利用方法**: [クレート README](../tagbase_memo/README.md)

## 支援・実験的クレート

### rokid_3dof（3DOF制御）
- **パス**: `rokid_3dof/`
- **役割**: 3自由度（3DOF）デバイスの制御
- **特徴**: HID API を使用したハードウェア連携
- **依存関係**: `hidapi`、`vqf-rs`（姿勢推定）

### ttf_overlap_remover（フォント最適化）
- **パス**: `ttf_overlap_remover/`
- **役割**: TTFフォントの重複除去と最適化
- **依存関係**: `rustybuzz`、`tiny-skia-path`

### font_rasterizer_example（サンプル）
- **パス**: `font_rasterizer_example/`
- **役割**: font_rasterizerクレートの使用例
- **特徴**: WebAssembly向けのビルド対応

## サンプルコード

### sample_codes/
- **showcase/**: プロジェクトのデモンストレーション
- **apng_gen/**: APNG生成ツールのサンプルコード
- **win_hello/**: Windows向けのHello Worldサンプル

## ドキュメント・リソース

### doc/
- **assets/**: ドキュメント用の画像・リソースファイル
- **memo/**: 開発メモ

### fonts/
- システムフォントの補完用フォントファイル
- `BIZUDMincho-Regular.ttf`: BIZ UDMincho フォント
- `NotoEmoji-Regular.ttf`: Noto Emoji フォント

### site/
- **役割**: mdBook を使用したドキュメントサイト
- **設定**: `book.toml` で日本語対応設定

## ビルド・開発環境

### 設定ファイル
- `Cargo.toml`: ワークスペース設定とメンバー定義
- `rust-toolchain.toml`: Rustツールチェインバージョン固定
- `mise.toml`、`mise.debug.toml`: 開発環境管理

### スクリプト
- `build-sites.sh`: サイトビルドスクリプト
- `dev-sites.sh`: 開発用サイトサーバー起動
- `release.sh`: リリース用ビルドスクリプト

## 技術スタック

- **言語**: Rust (Edition 2024)
- **グラフィックス**: WebGPU (wgpu)
- **ウィンドウ**: winit
- **フォントレンダリング**: 自前実装 + GPU処理
- **入力処理**: winit + 独自解析エンジン
- **3D数学**: glam
- **シリアライゼーション**: serde

## 設計思想

1. **モジュラー設計**: 機能を独立したクレートに分離
2. **GPU活用**: 高性能なレンダリングのためのWebGPU使用
3. **クロスプラットフォーム**: WebAssembly対応を含む
4. **アニメーション重視**: 滑らかなユーザーインターフェース
5. **3D空間**: 従来の2Dテキストエディタの概念を拡張

このアーキテクチャにより、従来のテキストエディタとは異なるユーザーエクスペリエンスの提供を目指しています。
