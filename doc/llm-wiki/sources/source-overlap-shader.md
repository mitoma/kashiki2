---
title: Source Summary - Overlap Shader
kind: source
status: production
updated: 2026-07-19
source_refs:
  - ../../font_rasterizer/src/shader/overlap_shader.wgsl
  - ../../../doc/ai-agent/plans/plan-changeRasterizerAlgorithm%20.md
related_pages:
  - ../concepts/text-rendering-pipeline.md
  - ../decisions/anti-aliasing-strategy.md
  - ../sources/source-vector-vertex-builder.md
---

# Source Summary - Overlap Shader

## 対象 source

- `font_rasterizer/src/shader/overlap_shader.wgsl`
- `doc/ai-agent/plans/plan-changeRasterizerAlgorithm .md`

## 要約

- vertex shader は `vertex_type` を `wait` と `triangle_type` に写像し、instance motion も同時に適用する
- `triangle_type` は bezier curve / bezier fill line / line を区別するためのフラグである
- fragment shader は multi render target へ color と count を出力し、`count.r` に winding、`count.g` に AA accum、`count.b` に AA sample count を積算する
- even-odd 用 entrypoint は常に正符号、non-zero 用 entrypoint は `@builtin(front_facing)` から `winding_sign` を決めて符号付きで積算する
- bezier / bezier_line / line の各ケースで、`wait` に基づく距離や範囲判定を行う

## 現行コードと関連計画の関係

- source には non-zero 用 `front_facing` entrypoint が存在する
- 一方で関連 plan では、重なり領域の AA 条件や overlap remover 除去まで含む段階的移行が整理されており、source 単体では未説明な運用意図を補っている

## wiki への影響

- AA strategy の「現在コード」と「計画上の到達点」を分けて扱える
- vector vertex builder との対応関係を trace できる