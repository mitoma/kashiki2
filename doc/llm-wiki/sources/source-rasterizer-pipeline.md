---
title: Source Summary - Rasterizer Pipeline
kind: source
status: production
updated: 2026-07-19
source_refs:
  - ../../font_rasterizer/src/rasterizer_pipeline.rs
related_pages:
  - ../concepts/text-rendering-pipeline.md
---

# Source Summary - Rasterizer Pipeline

## 対象 source

- `font_rasterizer/src/rasterizer_pipeline.rs`

## 要約

- `RasterizerPipeline` は文字とベクトル図形を GPU でラスタライズし、screen へ合成する最上位パイプラインである
- 構成は大きく `RasterizerRenderrer` による outline 生成、screen pass、background image pass、shader art pass に分かれる
- `Quarity` は oversampling 比率と固定解像度、上限制約付き very high を含む品質設定である
- modal 用に別の `RasterizerRenderrer` と outline texture を持ち、modal background の重ね描きに対応する
- `run_all_stage` は prepare / render / background / screen の順で各段を実行する
- `set_shader_art` と `update_buffer` は screen 側の背景表現と時間依存 uniform 更新を担う

## wiki への影響

- text rendering pipeline 概念の「どのステージがどこで走るか」を補強する
- shader art を Tier 3 の独立トピックとして切り出す起点になる