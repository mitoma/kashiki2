---
title: Anti-Aliasing Strategy
kind: decision
status: draft
updated: 2026-07-19
source_refs:
  - ../../memo/anti_aliasing.md
  - ../../../memories/repo/vector_vertex_aa_analysis.md
  - ../../../memories/repo/font_overlap_artifact_notes.md
  - https://blog.frost.kiwi/analytical-anti-aliasing/
related_pages:
  - ../concepts/text-rendering-pipeline.md
  - ./overlap-removal-strategy.md
  - ../sources/source-anti-aliasing.md
  - ../sources/source-frost-analytical-anti-aliasing.md
  - ../sources/source-vector-vertex-aa-analysis.md
  - ../sources/source-font-overlap-artifact-notes.md
  - ../sources/source-vector-vertex-builder.md
  - ../sources/source-overlap-shader.md
  - ../sources/source-outline-shader.md
---

# Anti-Aliasing Strategy

## 現行方針

- MSAA や SSAA ではなく analytical anti-aliasing を採用する
- `smoothstep` と `fwidth` による距離場ベースのエッジ解決を行う
- 直線、ベジェ曲線、ベジェ補助直線を頂点タイプで区別する
- 現時点のベースラインは `fwidth` ベースを維持しつつ、historical な改善案は source として保持する

## この判断の理由

- GPU 上で文字形状を高精度に扱いたい
- ベクトル形状と相性が良い
- conservative rasterization を含む shader 側の改善余地がある
- analytical anti-aliasing は、shape の数式や SDF を直接使って 1px 境界をフェードさせる発想として、この repo の shader 実装の理解に直結する

## historical な知見

- 凸性タグ付けで fill coverage を救済する案は、輪郭間の巻き相殺を壊すため棄却されている
- ベジエ接続部の AA 漏れに対しては、debug shader で弦クリップと MAX coverage ブレンドの改善が確認されている
- ただし debug での改善の一部は production 未反映で、fill rule や blend state への影響評価が残っている
- 現行コードでは `overlap_shader.wgsl` に even-odd 用と non-zero 用 entrypoint が分かれており、non-zero は `front_facing` による符号付き winding を使う
- `outline_shader.wgsl` は overlap count texture をサンプルして final alpha を決める resolve 段である

## overlap remover との関係

- even-odd ベースの経路では、font 側で重複除去して見た目を揃える戦略が併用されてきた
- non-zero / `front_facing` の経路が十分安定すれば overlap remover を不要化できる可能性があるが、現時点では移行途中として扱う

## 保留中の論点

- signed coverage 系の全面 rework を採用するか
- debug shader で試した改善を production にどう移すか
- overlap remover を不要化できる段階まで non-zero / front_facing 系の方針を進めるか
- `fwidth()` 近似と `length(dFdx, dFdy)` の精度差、あるいは per-object pixel size 計算をどう評価するか

## 反映済み source

- `font_rasterizer/src/vector_vertex.rs`
- `font_rasterizer/src/shader/overlap_shader.wgsl`
- `font_rasterizer/src/shader/outline_shader.wgsl`

## 次に深掘りすべき source

- `font_rasterizer/src/rasterizer_renderrer.rs`
- `font_rasterizer/src/shader/overlap_shader.debug.wgsl`
- `font_rasterizer/src/shader/outline_shader.debug.wgsl`