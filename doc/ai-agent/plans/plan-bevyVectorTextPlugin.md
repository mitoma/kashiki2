# Plan: Bevy Vector Text Plugin 統合

`font_rasterizer` の CPU glyph/vector 生成と WGSL 描画技術を用いて、Bevy 0.19 の ECS / Render World / Core2d render pipeline から利用できる `bevy_vector_text` という PlugIn crate を開発する。
ただし、これは汎用の Bevy Vector Text Plugin を提供することが目的であり、kashikishi を Bevy ベースに置き換えることを目的としているわけではない。

## 最終目標

- `VectorText` component に文字列、サイズ、色を設定すると glyph geometry が生成される
- `font_rasterizer` の canonical overlap/outline shader を Bevy の RenderDevice/RenderQueue で実行する
- EvenOdd / NonZero fill rule、Bézier coverage、アンチエイリアス、MSAA、resize に対応する

## 完了済み

### 1. Bevy crate と ECS API

- workspace に `bevy_vector_text` を追加
- Bevy 0.19.1 に対応
- `VectorTextPlugin` を追加
- `VectorText` component を追加
  - `text`
  - `font_size`
  - `color`
- `VectorTextGeometry` component を追加
- `ExtractComponentPlugin` で Main World から Render World へ抽出
- `ExtractSchedule` と `RenderSystems::Prepare/PrepareResources` を接続

### 2. font_rasterizer との CPU 統合

- `VectorVertexData` と `VectorVertex::into_data()` を追加
- `VectorTextFont` resource を追加
- `FontRepository`、`CharWidthCalculator`、`convert_char_to_vector_vertices` を再利用
- `VectorText.text` の変更時に glyph geometry を自動生成
- example で system font を検出して primary font を設定

### 3. Bevy GPU buffer

- `VectorTextGeometry` を vertex/index buffer に変換
- Bevy 0.19 の `RenderDevice::create_buffer_with_data` を使用
- vertex layout:
  - location 0: position
  - location 1: vertex type
  - location 2: color
- `VectorText.color` を GPU vertex と fragment shader へ接続

### 4. Core2d render pipeline

- Core2d の MainPass 後に draw system を追加
- `Camera2d` 付き独立 example を追加
- view target format を `Rgba8UnormSrgb` に統一
- MSAA 1/2/4/8 用 pipeline を用意
- glyph pass は sample count 1 の中間 texture に描画
- resolve pass は view の MSAA sample count に合わせて選択
- resize 時は `ViewTarget.main_texture().size()` から中間 texture を再生成

### 5. overlap/count/resolve

- color texture と `Rgba16Float` count texture の MRT を追加
- count target は additive blending
- fullscreen resolve shader で color/count texture を合成
- `front_facing` から winding sign を生成
- `VectorTextFillRule` resource を追加
  - `EvenOdd`
  - `NonZero`
- MSAA × fill rule ごとの resolve pipeline を生成

### 6. WGSL validation 対応

- `resolve_fragment_impl` が entry-point I/O struct を受けていた問題を修正
- helper は `vec2<f32>` の UV のみを受け取る構造へ変更
- helper に誤って残っていた `@fragment` annotation を削除
- 2分待ちの runtime example 実行で `ERROR`、`Validation`、`entry point cannot be called` がないことを確認

### 7. canonical shader 共有基盤

- `font_rasterizer::shader_sources` を追加
  - `OVERLAP`
  - `OUTLINE`
  - `SCREEN`
- `font_rasterizer::shader_contract` を追加
  - `OverlapUniforms`
  - `InstanceRaw`
- Bevy adapter が canonical shader source を受け取る接続点を追加
- canonical source を overlap / outline pipeline で実行し、fullscreen vertex のみ Bevy adapter shader から再利用
- overlap adapter で `VectorText.color.a` を引き継ぎ、outline output alpha に適用

### 8. Bevy uniform / instance buffer の準備

- `OverlapUniforms` を uniform buffer に upload し、overlap pass の bind group として接続
- `InstanceRaw` を geometry ごとの instance buffer として upload
- canonical shader の location 5〜13 に対応する instance vertex layout を追加
- overlap pipeline が uniform binding と instance attributes を使用

### 9. canonical overlap / outline pipeline と view cache

- Bevy adapter で canonical overlap / outline source を pipeline に接続し、fill rule ごとの entry point を選択
- `VectorText.color.a` を geometry attribute から overlap texture、outline output へ引き継ぐ
- outline fragment を `ViewTarget` に直接描画し、別の outline output texture は作成しない
- overlap / count texture と outline bind group を view entity ごとに cache
- extent、MSAA sample count、内部 texture format の変更時に cache を再作成
- view entity が despawn したら render cache を cleanup

## 検証済み

- `cargo check -p bevy_vector_text --examples`
- `cargo test --all`（失敗 0 件、slow test 1 件 ignored）
- `cargo clippy -p bevy_vector_text --all-targets -- -D warnings`
- `mise r check`
  - cargo fmt
  - cargo clippy --all
  - cargo clippy --tests --examples
- VS Code diagnostics エラーなし
- `cargo run -p bevy_vector_text --example basic` を十分な待ち時間で実行
- canonical overlap / outline pipeline と view cache 有効時に runtime の shader validation error がないことを確認

## 未完了タスク

### 1. geometry / instance の設計整理

現在の `VectorTextGeometry` は文字列全体を一つの geometry として保持する。canonical renderer は glyph geometry と instance buffer を分離するため、次を検討する。

- glyph 単位の vertex/index range
- 文字位置・scale・rotation の instance data
- `VectorText.font_size` を shader instance scale へ移動
- Bevy `GlobalTransform` と canonical model matrix の接続
- animation/motion flags の公開 API

### 2. visual regression

- `basic` example のスクリーンショット比較
- font_rasterizer PNG example との比較
- EvenOdd / NonZero の穴あき glyph
- Bézier 曲線の輪郭
- 小サイズ glyph の AA
- MSAA Off / 2 / 4 / 8
- window resize
- 複数 `VectorText` entity

### 3. ログと性能

現在 `font_rasterizer::vector_vertex` の INFO ログが example 実行時に大量に出る。Bevy example では通常 debug logging を抑え、必要時だけ有効化する。

GPU geometry buffer は現状毎 frame 再生成しているため、dirty tracking と cache を導入する。

## 次の実装順

1. geometry と instance を glyph 単位へ整理し、`GlobalTransform` と接続
2. visual regression を追加
3. geometry buffer の dirty tracking と性能比較を追加

## 既知の制約

- workspace の既存 wgpu は 30.0.1、Bevy 0.19.1 は wgpu 29 系を使用するため、wgpu 型を crate 境界で共有しない
- canonical shader は WGSL source の共有は可能だが、Rust 側の binding/layout/resource API は Bevy adapter として再構築する必要がある
- runtime example はウィンドウを開いたまま動作するため、検証時は十分な起動時間を与えた後、手動または terminal 停止で終了する
