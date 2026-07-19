---
title: Source Summary - Vector Vertex Builder
kind: source
status: production
updated: 2026-07-19
source_refs:
  - ../../font_rasterizer/src/vector_vertex.rs
  - ../../memo/E_midline_geometry_structure.md
related_pages:
  - ../components/vector-vertex-builder.md
  - ../concepts/text-rendering-pipeline.md
  - ../decisions/anti-aliasing-strategy.md
---

# Source Summary - Vector Vertex Builder

## 対象 source

- `font_rasterizer/src/vector_vertex.rs`
- `doc/memo/E_midline_geometry_structure.md`

## 要約

- `VectorVertexBuilder` は `OutlineBuilder` を実装し、アウトライン命令列を `vertex` と `index` に変換する
- `line_to` は原点 L と line 専用終点を使った直線三角形を作り、`quad_to` は制御点、Bezier fill 専用頂点、終点 B / L を使って曲線部と補助直線部を分離する
- `close` はサブパス単位で重心原点を追加し、予約 index 0 / 1 をサブパス専用 origin に置換する
- `FlipFlop` から 0..8 の `vertex_type` を作り、shader 側の `wait` と `triangle_type` 判定に繋ぐ
- `doc/memo/E_midline_geometry_structure.md` は直線三角形で `wait` がどう内挿され、fragment shader がどう `is_line` 判定を行うかを補助説明している

## wiki への影響

- vector vertex builder component と AA strategy をコード面から補強できる
- shader source を読む前提知識として `vertex_type` / `wait` / `triangle_type` の意味を固定できる