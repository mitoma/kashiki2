# Plan: RasterizerRenderrer を外部テクスチャ受け取り型に変更

`RasterizerRenderrer` から `outline_texture` を削除し、`render()` メソッドで外部からターゲットテクスチャの `TextureView` を受け取るよう変更します。`overlap_stage` で使う内部テクスチャ (`overlap_texture`, `overlap_count_texture`) は引き続き内部で保持します。

## Steps

### 1. `RasterizerRenderrer` 構造体から `outline_texture` フィールドを削除

[`rasterizer_renderrer.rs`](c:\Users\mutet\workspace\kashiki2\font_rasterizer\src\rasterizer_renderrer.rs) の `RasterizerRenderrer` 構造体から `outline_texture: ScreenTexture` フィールドを削除し、`new()` メソッドでの `outline_texture` の初期化処理も削除。

### 2. `render()` メソッドに `target_view` パラメータを追加

`render()` メソッドのシグネチャを `pub fn render(&self, encoder: &mut wgpu::CommandEncoder, buffers: Buffers, target_view: &wgpu::TextureView)` に変更し、`outline_stage()` にも `target_view` を渡すよう修正。

### 3. `outline_stage()` メソッドを外部テクスチャへレンダリング

`outline_stage()` メソッドに `target_view: &wgpu::TextureView` パラメータを追加し、内部で `create_view()` していた箇所を削除して引数の `target_view` を直接使用するよう変更。

### 4. `outline_render_pipeline` のターゲットフォーマットを可変に対応

`new()` メソッドに `target_texture_format: wgpu::TextureFormat` パラメータを追加し、`outline_render_pipeline` の `ColorTargetState::format` を `outline_texture.texture_format` から引数の値に変更。

### 5. `RasterizerPipeline` での `RasterizerRenderrer` 作成と呼び出しを更新

[`rasterizer_pipeline.rs`](c:\Users\mutet\workspace\kashiki2\font_rasterizer\src\rasterizer_pipeline.rs) の `new()` で `RasterizerRenderrer::new()` に `screen_texture_format` を渡し、`outline_texture` フィールド用の内部テクスチャ (`ScreenTexture`) を2つ作成して保持。`render()` 呼び出し時にそのテクスチャの `view` を渡すよう変更。

### 6. `screen_stage()` での `outline_texture` 参照を内部テクスチャに変更

`screen_stage()` メソッドで `self.rasterizer_renderrer.outline_texture` を参照している箇所を、新たに `RasterizerPipeline` が保持する内部テクスチャ (`outline_texture`, `outline_texture_for_modal`) への参照に変更。

## Further Considerations

### 1. `prepare()` メソッドの `update_textures()` 呼び出し

現在 `prepare()` で毎フレーム `outline_bind_group.update_textures()` を呼んでいますが、`overlap_texture` と `overlap_count_texture` は不変なので、この呼び出しは `new()` 時の1回のみで十分です。今回は最適化しない方針ですが、将来的に検討の余地があります。

### 2. リサイズ処理

`RasterizerPipeline` のリサイズ時には、新たに保持する `outline_texture` / `outline_texture_for_modal` も再作成が必要です。現在は `RasterizerPipeline` 全体を再作成しているため自動的に対応されます。

### 3. テクスチャフォーマット整合性

`outline_render_pipeline` のターゲットフォーマットと実際にレンダリングする `target_view` のフォーマットが一致している必要があります。呼び出し側で適切なフォーマットを保証する責務が発生します。
