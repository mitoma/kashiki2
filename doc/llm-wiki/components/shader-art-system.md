---
title: Shader Art System
kind: component
status: draft
updated: 2026-07-19
source_refs:
  - ../../font_rasterizer/src/builtin_shader_art.rs
  - ../../font_rasterizer/src/rasterizer_pipeline.rs
  - ../../kashikishi/src/main.rs
  - ../../sample_codes/apng_gen/src/lib.rs
related_pages:
  - ../sources/source-shader-art-system.md
  - ../concepts/text-rendering-pipeline.md
---

# Shader Art System

## 概要

shader art system は、screen 背景を WGSL のフルスクリーンシェーダーで描くための仕組みで、テキスト描画本体とは独立した背景表現レイヤーである。

## 主要要素

- `builtin_shader_art.rs`
  - name / display_name / source を持つ組み込み shader 一覧
- `RasterizerPipeline::set_shader_art`
  - 文字列ソースから shader module と pipeline を生成する
- `screen_background_image_stage`
  - shader art がある場合は背景画像より優先して描画する
- `main.rs`
  - config の `background_shader` 名から組み込み shader を選ぶ

## 運用上の特徴

- `background_image` と排他的ではないが、実行時は shader art が優先される
- `update_buffer` で時刻、画面サイズ、背景色を uniform として更新する
- `sample_codes/apng_gen` などでは現状 `shader_art: None` のため未利用経路になっている

## 未整理の論点

- ユーザー編集 shader の保存導線
- wiki 上で各組み込み shader 個別の表現意図を別ページ化するか