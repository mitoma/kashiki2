---
title: Source Summary - Easy Scalable Text Rendering on the GPU
kind: source
status: production
updated: 2026-07-19
source_refs:
  - https://medium.com/@evanwallace/easy-scalable-text-rendering-on-the-gpu-c3f4d782c5ac
source_url: https://medium.com/@evanwallace/easy-scalable-text-rendering-on-the-gpu-c3f4d782c5ac
retrieved_at: 2026-07-19
raw_capture: none
license_note: public repo には原文本文を保存せず、要約と URL のみを保持する
related_pages:
  - ../concepts/text-rendering-pipeline.md
  - ../decisions/anti-aliasing-strategy.md
  - ../sources/source-rasterizer-pipeline.md
  - ../sources/source-overlap-shader.md
  - ../sources/source-outline-shader.md
---

# Source Summary - Easy Scalable Text Rendering on the GPU

## 対象 source

- `Easy Scalable Text Rendering on the GPU` by Evan Wallace
- URL: https://medium.com/@evanwallace/easy-scalable-text-rendering-on-the-gpu-c3f4d782c5ac

## 要約

- GPU はポリゴンを直接描けないため、グリフ輪郭を三角形集合へ落とし込み、winding number を使って内部判定する
- 直線輪郭は任意の原点から各辺への三角形 fan で表せる
- 二次ベジェ曲線は 1 セグメントごとに 1 つの三角形 correction として扱え、barycentric 座標上の不等式 `(s / 2 + t)^2 < t` で内部判定できる
- こうして fill 段を GPU 上で完結させ、解像度非依存かつテクスチャキャッシュ不要な text rendering ができる
- anti-aliasing は複数サンプルの accumulation と subpixel AA を組み合わせて改善できる

## このリポジトリへの対応

- `RasterizerPipeline` の「overlap を積み、outline を resolve して screen に出す」三段構成は、この説明と強く対応する
- `VectorVertexBuilder` が作る fan とベジェ補助直線 / 曲線三角形の分割は、記事中の polygon fill と curve correction の発想に対応する
- `overlap_shader.wgsl` の `wait` / `triangle_type` と `outline_shader.wgsl` の resolve は、記事で言う GPU 側の内部判定と anti-aliasing 実装を repo 向けに具体化したものとみなせる

## 現行実装との違い

- 記事は even-odd 系の説明が中心だが、この repo では non-zero / `front_facing` 経路や overlap remover との併用もある
- 記事では subpixel anti-aliasing まで扱うが、この repo の現行 source は別の AA 実装上の制約と改善履歴を持つ

## 注目キーワード

- winding number
- triangle fan
- quadratic bezier correction
- barycentric coordinates
- additive accumulation
- subpixel anti-aliasing