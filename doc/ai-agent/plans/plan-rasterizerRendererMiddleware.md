# Plan: RasterizerRenderrer を wgpu Middleware パターンへリファクタリング

現在の `RasterizerRenderrer` は「リソース管理」と「レンダリング」が混在し、`device` を render 時に受け取るなど wgpu の推奨パターンから逸脱しています。prepare/render の責務を明確に分離し、bind group 生成を事前に行う形式へ変更します。

## Steps

1. **`OutlineBindGroup` を状態保持型へ統一** - `outline_bind_group.rs` の `to_bind_group()` を廃止し、`OverlapBindGroup` 同様に `bind_group` メンバを保持、`update_textures()` メソッドで更新する形式へ変更

2. **`RasterizerRenderrer::prepare()` メソッドを新設** - `queue: &wgpu::Queue`, `view_proj`, `buffers` を受け取り、`overlap_bind_group.update_buffer(queue)` と `outline_bind_group.update_textures()` を実行、bind group とバッファの GPU 転送を完了させる

3. **`RasterizerRenderrer::render()` の簡素化** - シグネチャから `device`, `queue`, `view_proj`, `buffers` を削除し `encoder: &mut wgpu::CommandEncoder` のみ受け取る形式へ変更、内部では `overlap_stage()` / `outline_stage()` のみ呼び出す

4. **`outline_stage()` から `device` 引数を削除** - 事前に `prepare()` で bind group を生成済みのため `device` 不要、`self.outline_bind_group.bind_group` を直接参照

5. **呼び出し側の修正** - `rasterizer_pipeline.rs` などで `renderrer.prepare(queue, view_proj, buffers); renderrer.render(encoder);` の2段階呼び出しへ変更

## Further Considerations

1. **`ScreenTexture` のリサイズ対応** - 現在は固定サイズ前提ですが、将来ウィンドウリサイズ時に `recreate()` メソッドを追加する必要があるか？
  - 一旦は現状から変更しない。実装整理が進んできた後に検討する
2. **BindGroup 更新の最適化** - `outline_bind_group` は毎フレーム生成不要な場合があるため、dirty flag を導入するか？
  - 一旦は現状から変更しない。実装整理が進んできた後に検討する
3. **エラーハンドリング** - `prepare()` で GPU 転送失敗時の戻り値を `Result<(), Error>` にするか？
  - `thiserror` クレートを導入して独自エラー型を定義する 
