# Index

このファイルは wiki の入口です。新しい query や更新では、まずここを見て関連ページを特定します。

## Core Pages

- [README.md](README.md) この wiki の目的とレイヤー
- [schema.md](schema.md) 更新規約とページ種別
- [log.md](log.md) ingest / query / lint の履歴

## Concepts

- [concepts/text-rendering-pipeline.md](concepts/text-rendering-pipeline.md) GPU テキスト描画の全体像
- [concepts/layout-engine.md](concepts/layout-engine.md) 折り返し、幅計算、preedit を含むレイアウト概念

## Decisions

- [decisions/anti-aliasing-strategy.md](decisions/anti-aliasing-strategy.md) 現行 AA と保留案
- [decisions/unicode-segmentation.md](decisions/unicode-segmentation.md) ICU4X 導入方針
- [decisions/preedit-source-of-truth.md](decisions/preedit-source-of-truth.md) preedit 配置の唯一の正

## Workflows

- [workflows/ingest-workflow.md](workflows/ingest-workflow.md) 新 source の取り込み手順

## Sources

- [sources/source-project-overview.md](sources/source-project-overview.md) [../project.md](../project.md) の要約
- [sources/source-text-buffer-wrapping.md](sources/source-text-buffer-wrapping.md) [../text_buffer_wrapping.md](../text_buffer_wrapping.md) の要約
- [sources/source-anti-aliasing.md](sources/source-anti-aliasing.md) [../memo/anti_aliasing.md](../memo/anti_aliasing.md) の要約

## Backlog

### Tier 1

- `doc/memo/font.md` の source 化
- `/memories/repo/text_buffer_design.md` の wiki 反映
- `/memories/repo/vector_vertex_aa_analysis.md` の historical source 化
- `/memories/repo/font_overlap_artifact_notes.md` の historical / decision 反映

### Tier 2

- `font_rasterizer/src/rasterizer_pipeline.rs` の source 化
- `font_rasterizer/src/vector_vertex.rs` の source 化
- `font_rasterizer/src/shader/overlap_shader.wgsl` の source 化
- `font_rasterizer/src/shader/outline_shader.wgsl` の source 化
- `text_buffer/src/editor.rs` の source 化
- `text_buffer/src/layout.rs` の source 化
- `text_buffer/src/action.rs` の source 化

### Tier 3

- shader art 系ページの追加
- overlap remover の component / decision 化
- World / scene 構成の concepts 化

## 運用メモ

- 新規ページを追加したら、この index に追記する
- backlog から消すときは、対応する source / concept / decision のどこに反映したかを log に残す