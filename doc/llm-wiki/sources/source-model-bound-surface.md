---
title: Source Model Bound Surface
kind: source
status: production
updated: 2026-11-06
source_refs:
  - ../../ai-agent/plans/model-surface-background.md
  - ../../../ui_support/src/layout_engine/model.rs
  - ../../../ui_support/src/layout_engine/default_world.rs
  - ../../../ui_support/src/render_state.rs
  - ../../../font_rasterizer/src/rasterizer_pipeline.rs
  - ../../../font_rasterizer/src/shader/screen_shader.wgsl
  - ../../../kashikishi/e2e/scenarios/tagbase_memo.jsonl
related_pages:
  - ../decisions/model-bound-surface.md
  - ../concepts/text-rendering-pipeline.md
  - ../workflows/model-surface-e2e.md
---

# Source: Model Bound Surface

## 要約

設計は [model-surface-background plan](../../ai-agent/plans/model-surface-background.md)、実装は `ModelSurfaceStyle` と `ModelRenderBatch`、`DefaultWorld::render_batches`、`model_batches_to_buffers`、`RasterizerPipeline::run_all_stage` に分かれる。

World はモデル順と surface/content の境界を保持する。RenderState は各モデルを順序付き `Buffers` にする。Windfoil は全 batch を一回の ordered render に集約し、一回の premultiplied-alpha render pass と resolve で outline texture を生成する。Legacy は coverage/count texture の意味を保つため per-batch resolve と全画面 composite を継続する。normal と modal の出力 texture は独立し、modal blur を保つ。

TextEdit の面は `ThemeBackground`、`Color([f32; 3])`、`None` を持ち、モデルの現在位置・回転・world scale・現在 bound と、glyph easing 由来の camera detail を instance に反映する。Card profile は glyph に `IGNORE_CAMERA` を設定するため、surface にも同じ detail が必要である。矩形 SVG は RenderState 作成時に一度登録されるため、アプリ callback の登録漏れがない。

Card は通常 World model ではなく ModalWorld overlay として glyph/vector を供給する。overlay API も surface instances を返し、RenderData の overlay batch に追加しなければ面が描画されない。

## 検証上の注意

`mise r e2e` は GPU を用いる ignored test であり、PNG と text snapshot の比較をする。背景面が画面背景と同色の場合、PNG だけでは面の存在を見分けにくい。調査では一時的な明色面を使って geometry と背面文字の遮蔽を確認し、確認後にテーマ背景へ戻して golden 比較する。

E2E シナリオは [tagbase_memo.jsonl](../../../kashikishi/e2e/scenarios/tagbase_memo.jsonl) で、model swap 中の snapshot と検索・meta card の状態を記録する。

## 性能計測

同一 debug 環境の headed E2E で `render 4: run all stage` を計測した。command palette は旧約 20 ms/frame → Windfoil ordered 約 1 ms/frame、tagbase memo は旧約 6 ms/frame → 1 ms 未満。phase timer は GPU timestamp ではなく CPU 側 encoding を含むため、絶対値ではなく同じ環境・シナリオでの比較として扱う。