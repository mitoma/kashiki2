# プロジェクト構造

## 概要

炊紙（kashikishi）は三次元空間上でテキストを編集できるテキストエディタです。  
Rust & WebGPU(wgpu)で開発されており、従来のテキストエディタにはない滑らかな編集体験を提供することを目指しています。

## アーキテクチャ

このプロジェクトはRustのワークスペース構成を採用し、機能ごとに独立したクレートで構成されています。以下に主要なクレートとその役割を示します。

## 主要クレート

### kashikishi（メインアプリケーション）
- **パス**: `kashikishi/`
- **役割**: メインの実行ファイルとなるアプリケーション
- **依存関係**: 他の全てのクレートを統合し、最終的なテキストエディタを提供

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
```

### font_rasterizer（フォントレンダリング）
- **パス**: `font_rasterizer/`
- **役割**: フォントのラスタライゼーションとレンダリング処理
- **特徴**: WebGPU(wgpu)を使用したGPUベースのフォント描画
- **ライブラリタイプ**: `cdylib`、`rlib`（WebAssemblyにも対応）

#### 描画バックエンド

`RenderingBackend` で以下の2方式を選択できます。既定値は `Legacy` です。

- **Legacy**: 従来の三角形ラスタライズ、巻き数テクスチャ、輪郭抽出による描画。対応 GPU では conservative rasterization を使用します。
- **Windfoil**: 有向二次ベジェ曲線を storage buffer に格納し、水平バンドと解析的な曲線積分で被覆率を計算します。距離依存の変形には曲線の事前投影を使用します。

操作メニューの「描画方式の変更」で、アプリを再起動せずに切り替えられます。
対応するアクションは `system:change-rendering-backend-ui`、直接指定するアクションは
`system:change-rendering-backend`（引数 `legacy` / `windfoil`）です。
選択は `EditorSettings.rendering_backend` に `Legacy` / `Windfoil` として保存されます。
この項目がない既存の設定ファイルでは `Legacy` が選択されます。

`RasterizerRenderrer` が両バックエンドを仲介し、旧処理は `legacy_renderrer.rs`、
Windfoil は `windfoil_renderrer.rs` に分離しています。
字形と SVG は既存の三角形データを共有し、補助三角形を除いた輪郭辺から Windfoil 用の曲線を復元します。
そのため既存の字形キャッシュ形式は変更せず、Windfoil の GPU 曲線バッファは初回使用時に生成・保持します。
旧方式の GPU バッファも保持するため、切り替え時のフォント再変換は不要ですが、Windfoil 使用後は両方式のバッファ分のメモリを使用します。

切り替え時には通常画面とモーダルの描画パイプラインを再生成します。
背景画像とシェーダーアートを引き継ぎ、リサイズ・品質変更・塗り規則変更後もバックエンドの選択を維持します。
WebAssembly でも storage buffer を使用できる WebGPU の default limits を要求します。

ライブラリ利用時は `RasterizerPipeline::new_with_backend` または
`RasterizerRenderrer::new_with_backend` で指定します。既存の `new` は `Legacy` を使用します。
PNG 出力は `render_vector_vertex_to_png_with_backend` とその非同期版で明示的に選択できます。

GPU 回帰テストは通常のテストでは無効化されているため、必要に応じて次を実行します。

```powershell
cargo test -p font_rasterizer windfoil_ --lib -- --include-ignored
cargo test -p font_rasterizer rendering_backends_switch_with_shared_buffers --lib -- --ignored
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

### ui_support（UI支援）
- **パス**: `ui_support/`
- **役割**: ユーザーインターフェースの支援機能
- **依存関係**: `nenobi`（外部ライブラリ）を使用

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
