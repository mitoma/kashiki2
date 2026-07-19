---
title: Source Summary - Shader Art System
kind: source
status: production
updated: 2026-07-19
source_refs:
  - ../../font_rasterizer/src/builtin_shader_art.rs
  - ../../font_rasterizer/src/rasterizer_pipeline.rs
  - ../../kashikishi/src/main.rs
  - ../../sample_codes/apng_gen/src/lib.rs
related_pages:
  - ../components/shader-art-system.md
  - ../concepts/text-rendering-pipeline.md
---

# Source Summary - Shader Art System

## 対象 source

- `font_rasterizer/src/builtin_shader_art.rs`
- `font_rasterizer/src/rasterizer_pipeline.rs`
- `kashikishi/src/main.rs`
- `sample_codes/apng_gen/src/lib.rs`

## 要約

- 組み込み shader art は `name`, `display_name`, `source` を持つ静的一覧として定義される
- `RasterizerPipeline` は `set_shader_art` で shader module と専用 pipeline を作り、背景描画段で背景画像より優先して実行する
- `update_buffer` は shader art 向け uniform に time、window size、背景色を流し込む
- `kashikishi` 本体は config の `background_shader` 名から組み込み shader を引く
- `sample_codes/apng_gen` は現状 shader art を使っていないが、背景画像との並列な入力経路を持つ

## wiki への影響

- shader art を rendering pipeline 本体から切り分けて説明できる
- 設定ファイル、runtime、サンプル利用の接点を source で固定できる