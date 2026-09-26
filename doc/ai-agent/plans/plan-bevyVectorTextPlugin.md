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
- 現在の runtime shader は、binding/instance 契約が異なるため既存の Bevy adapter shader を使用中

## 検証済み

- `cargo check -p bevy_vector_text --examples`
- `cargo clippy -p bevy_vector_text --all-targets -- -D warnings`
- `mise r check`
  - cargo fmt
  - cargo clippy --all
  - cargo clippy --tests --examples
- VS Code diagnostics エラーなし
- `cargo run -p bevy_vector_text --example basic` を十分な待ち時間で実行
- runtime の shader validation error がないことを確認

## 未完了タスク

### 1. canonical overlap shader の実 pipeline 移植

`font_rasterizer/src/shader/overlap_shader.wgsl` は以下を要求する。

- group 0 / binding 0 の `OverlapUniforms`
- vertex location 0/1 の geometry
- instance location 5〜13 の `InstanceRaw`
- `vs_main`
- `fs_main_even_odd`
- `fs_main_non_zero`
- color/count の MRT

Bevy 側で必要な実装:

- `shader_contract::OverlapUniforms` から uniform buffer を作成
- bind group layout と bind group を作成
- `InstanceRaw` を instance vertex buffer として upload
- location 5〜13 の `VertexBufferLayout` を追加
- canonical source を `Shader::from_wgsl` へ渡す
- fragment entry point を fill rule に応じて選択

### 2. canonical outline shader の移植

- `outline_shader.wgsl` を canonical source として使用
- overlap color/count texture を bind group に接続
- outline output texture を作成
- EvenOdd / NonZero entry point を切り替え
- resolve pass の簡易 count 計算を canonical outline resolve に置換

### 3. pipeline resource lifecycle

現在は draw system 内で中間 texture を毎 frame 作成している。将来的には以下へ変更する。

- view entity ごとの render resource cache
- size / format / MSAA が変わったときだけ再作成
- entity despawn 時の resource cleanup
- multiple camera 対応

### 4. geometry / instance の設計整理

現在の `VectorTextGeometry` は文字列全体を一つの geometry として保持する。canonical renderer は glyph geometry と instance buffer を分離するため、次を検討する。

- glyph 単位の vertex/index range
- 文字位置・scale・rotation の instance data
- `VectorText.font_size` を shader instance scale へ移動
- Bevy `GlobalTransform` と canonical model matrix の接続
- animation/motion flags の公開 API

### 5. visual regression

- `basic` example のスクリーンショット比較
- font_rasterizer PNG example との比較
- EvenOdd / NonZero の穴あき glyph
- Bézier 曲線の輪郭
- 小サイズ glyph の AA
- MSAA Off / 2 / 4 / 8
- window resize
- 複数 `VectorText` entity

### 6. ログと性能

現在 `font_rasterizer::vector_vertex` の INFO ログが example 実行時に大量に出る。Bevy example では通常 debug logging を抑え、必要時だけ有効化する。

GPU resource は現状毎 frame 再生成しているため、canonical shader 移植後に dirty tracking と cache を導入する。

## 次の実装順

1. `OverlapUniforms` の Bevy uniform buffer / bind group を追加
2. `InstanceRaw` 用の Bevy instance buffer と layout を追加
3. canonical `overlap_shader.wgsl` へ pipeline を切り替え、runtime validation を修正
4. canonical `outline_shader.wgsl` と texture bind group を移植
5. outline pass と view target resolve を統合
6. render resource cache と resize lifecycle を整理
7. visual regression と性能比較を追加

## 既知の制約

- workspace の既存 wgpu は 30.0.1、Bevy 0.19.1 は wgpu 29 系を使用するため、wgpu 型を crate 境界で共有しない
- canonical shader は WGSL source の共有は可能だが、Rust 側の binding/layout/resource API は Bevy adapter として再構築する必要がある
- runtime example はウィンドウを開いたまま動作するため、検証時は十分な起動時間を与えた後、手動または terminal 停止で終了する
