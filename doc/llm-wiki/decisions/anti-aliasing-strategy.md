---
title: Anti-Aliasing Strategy
kind: decision
status: draft
updated: 2026-09-13
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
- overlap pass では `count.r` に符号付き winding、`count.g` に符号付き edge coverage の積算、`count.b` に edge 寄与数を加算する
- outline pass では `abs(count.g) / (abs(count.b) / UNIT)` を edge alpha として復元し、fill rule に応じて inside / outside を解決する
- 現時点のベースラインはこの signed accumulation と `fwidth` ベースの analytical AA を維持する

## この判断の理由

- GPU 上で文字形状を高精度に扱いたい
- ベクトル形状と相性が良い
- conservative rasterization を含む shader 側の改善余地がある
- analytical anti-aliasing は、shape の数式や SDF を直接使って 1px 境界をフェードさせる発想として、この repo の shader 実装の理解に直結する

## historical な知見

- 凸性タグ付けで fill coverage を救済する案は、輪郭間の巻き相殺を壊すため棄却されている
- ベジエ接続部の AA 漏れに対しては、弦側を除外した signed accumulation と edge 寄与数の平均化を production / debug の両方へ反映した
- 過去に debug shader で試した MAX coverage ブレンドは、現行の平均化と fill rule の構造が異なるため採用しない
- 現行コードでは `overlap_shader.wgsl` に even-odd 用と non-zero 用 entrypoint が分かれており、non-zero は `front_facing` による符号付き winding を使う
- `outline_shader.wgsl` は overlap count texture をサンプルして final alpha を決める resolve 段である

## overlap remover との関係

- even-odd ベースの経路では、font 側で重複除去して見た目を揃える戦略が併用されてきた
- non-zero / `front_facing` の経路が十分安定すれば overlap remover を不要化できる可能性があるが、現時点では移行途中として扱う

## 保留中の論点

- signed coverage の寄与数平均化が conservative rasterization の隣接三角形重複に依存するため、別の rasterization 条件でも品質を維持できるか
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