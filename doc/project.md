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

### font_rasterizer（フォントレンダリング）
- **パス**: `font_rasterizer/`
- **役割**: フォントのラスタライゼーションとレンダリング処理
- **特徴**: WebGPU(wgpu)と Windfoil の解析的な曲線積分を使用したフォント描画
- **ライブラリタイプ**: `cdylib`、`rlib`（WebAssemblyにも対応）

#### Windfoil 描画

[Windfoil](https://github.com/texel-org/windfoil-algorithm) の
[アルゴリズム仕様](https://github.com/texel-org/windfoil-algorithm/blob/main/docs/ALGORITHM.md)
に基づいて実装している。元の実装のコピーではなく、このプロジェクトの座標系・モーション・合成方式へ組み込んだ実装。

- `font_rasterizer/src/windfoil.rs` は既存パスから有向の直線・二次曲線を取り出し、x/y の極値で単調分割する。1〜64 の水平バンドに分類し、右端の降順で曲線を並べる。
- `font_rasterizer/src/shader/windfoil.wgsl` は文字・SVG ごとの矩形を描画し、storage buffer の曲線からピクセル内の巻き数積分を解析的に求める。バンド境界をまたぐピクセルは各バンドの交差範囲だけを積分する。
- non-zero は積分の絶対値を 1 に制限し、even-odd は周期 2 の三角波で被覆率を求める。AA 無効時は中心点で巻き数を評価する。
- 座標に依存しないモーション・カメラ変換では局所座標と画面微分を使う。座標依存モーションは元の制御点を変形し、画面座標で再分割・積分する。この経路は行バンドによる枝刈りを使わないため重い。
- パス・巻き数をテクスチャへ焼き込まない。描画色の合成と従来の straight-alpha 出力のためのテクスチャは残し、画質設定・モーダル・背景・PNG 出力の API を維持している。
- WebAssembly でも WebGPU の storage buffer が必要。WebGL2 のフォールバックは使用できない。conservative rasterization は不要。

精度の制約として、複数の巻き数レベルをまたぐ重複・交差部分では、巻き数積分の折り畳みは真の塗り面積と一致しない場合がある。
通常経路の回転・せん断では `fwidth` による軸平行な局所ボックスを使用する。
透視変換後の曲線は一般に有理曲線なので、座標依存モーション経路での制御点投影は近似となる。
既存の三次曲線から二次曲線への近似と、ほぼ直線な曲線の簡略化は維持している。f32 の座標精度の制限も残る。
提案元の小サイズ用 ink-profile 近似や点サンプル exact mode は導入せず、AA 有効時は解析積分を使う。実性能の改善は保証しない。

GPU 回帰テスト（WebGPU 対応 GPU が必要）:

```powershell
cargo test -p font_rasterizer windfoil_gpu_coverage_regressions --lib -- --ignored
```

サブピクセル矩形・細線・曲線の独立サンプル比較・バンド境界・塗り規則・座標依存モーション・AA 無効・空パスを検証する。
`FONT_RASTERIZER_DEBUG_SHADER` 有効時は、既存の `overlap_shader.debug.wgsl` のモーション計算と `windfoil.wgsl` をディスクから読む。

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
