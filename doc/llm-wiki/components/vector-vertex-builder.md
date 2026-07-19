---
title: Vector Vertex Builder
kind: component
status: draft
updated: 2026-07-19
source_refs:
  - ../../font_rasterizer/src/vector_vertex.rs
  - ../../memo/E_midline_geometry_structure.md
related_pages:
  - ../concepts/text-rendering-pipeline.md
  - ../sources/source-vector-vertex-builder.md
---

# Vector Vertex Builder

## 概要

`VectorVertexBuilder` は TTF / SVG のアウトライン入力を受け、GPU で扱う `VectorVertex` と index 列へ変換する `OutlineBuilder` 実装である。

## 主要責務

- `move_to`, `line_to`, `quad_to`, `curve_to`, `close` を通じて三角形列を構成する
- `FlipFlop` から `vertex_type` へ変換し、shader 側の `wait` / `triangle_type` 判定に必要な情報を埋め込む
- close 時にサブパスごとの重心原点を追加し、予約 index 0 / 1 をサブパス専用 origin に置換する
- 2 次 / 3 次ベジエと補助直線用の頂点共有問題を避けるため、Bezier fill 専用頂点を持つ

## 重要な実装上の点

- `current_index` は 0 / 1 を原点予約として 2 相当から進める
- `quad_to` は制御点、補助直線始終点、終点 B / L を分けて push する
- `CoordinateSystem` と `VertexBuilderOptions` により SVG / Font 座標系やスケール変換を切り替える

## 未整理の論点

- `vector_vertex.rs` と shader 側の `vertex_type` 対応表をどこまで単独ページ化するか
- `curve_to` の 3 次ベジエ近似の品質評価