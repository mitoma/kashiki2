---
title: Anti-Aliasing Strategy
kind: decision
status: draft
updated: 2026-07-19
source_refs:
  - ../../memo/anti_aliasing.md
related_pages:
  - ../concepts/text-rendering-pipeline.md
  - ../sources/source-anti-aliasing.md
---

# Anti-Aliasing Strategy

## 現行方針

- MSAA や SSAA ではなく analytical anti-aliasing を採用する
- `smoothstep` と `fwidth` による距離場ベースのエッジ解決を行う
- 直線、ベジェ曲線、ベジェ補助直線を頂点タイプで区別する

## この判断の理由

- GPU 上で文字形状を高精度に扱いたい
- ベクトル形状と相性が良い
- conservative rasterization を含む shader 側の改善余地がある

## 保留中の論点

- signed coverage 系の全面 rework を採用するか
- debug shader で試した改善を production にどう移すか
- MAX coverage や historical な失敗案を decision 群へどう整理するか

## 次に反映すべき source

- `font_rasterizer/src/vector_vertex.rs`
- `font_rasterizer/src/shader/overlap_shader.wgsl`
- `/memories/repo/vector_vertex_aa_analysis.md`
- `/memories/repo/font_overlap_artifact_notes.md`