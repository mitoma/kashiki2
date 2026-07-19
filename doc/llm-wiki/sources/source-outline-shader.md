---
title: Source Summary - Outline Shader
kind: source
status: production
updated: 2026-07-19
source_refs:
  - ../../font_rasterizer/src/shader/outline_shader.wgsl
  - ../../../doc/ai-agent/plans/plan-changeRasterizerAlgorithm%20.md
related_pages:
  - ../concepts/text-rendering-pipeline.md
  - ../decisions/anti-aliasing-strategy.md
  - ../sources/source-overlap-shader.md
---

# Source Summary - Outline Shader

## 対象 source

- `font_rasterizer/src/shader/outline_shader.wgsl`
- `doc/ai-agent/plans/plan-changeRasterizerAlgorithm .md`

## 要約

- outline shader は outline texture の色と overlap count texture の値を読んで最終 alpha を resolve する
- `fs_main_even_odd` は count のパリティで inside/outside を決め、AA accum と count から alpha を作る
- `fs_main_non_zero` は winding が非ゼロかどうかで inside を判定し、符号付き count の絶対値から alpha を作る
- 現行 source は even-odd / non-zero を entrypoint で分離し、resolve 段で fill rule を切り替える構造を持つ

## 現行コードと関連計画の関係

- plan では non-zero のエッジ条件をさらに厳密化し、重なり深部の偽 AA を抑える条件まで整理されている
- source summary では current code を優先しつつ、より進んだ AA 条件設計が別文書に存在することを残す

## wiki への影響

- fill rule の差分位置を text rendering pipeline の最終段として固定できる
- anti-aliasing strategy の resolve 側論点を source で追跡できる