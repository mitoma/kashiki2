---
title: Source Summary - AAA Analytical Anti-Aliasing
kind: source
status: production
updated: 2026-07-19
source_refs:
  - https://blog.frost.kiwi/analytical-anti-aliasing/
source_url: https://blog.frost.kiwi/analytical-anti-aliasing/
retrieved_at: 2026-07-19
raw_capture: none
license_note: public repo には原文本文や画像を保存せず、要約と URL のみを保持する
related_pages:
  - ../decisions/anti-aliasing-strategy.md
  - ../concepts/text-rendering-pipeline.md
  - ./source-anti-aliasing.md
  - ./source-overlap-shader.md
---

# Source Summary - AAA Analytical Anti-Aliasing

## 対象 source

- `AAA - Analytical Anti-Aliasing`
- URL: https://blog.frost.kiwi/analytical-anti-aliasing/

## 要約

- 記事は SSAA、MSAA、FXAA などを比較した上で、shape の数式や SDF を使って境界をちょうど 1px 分だけフェードさせる analytical anti-aliasing を説明している
- analytical AA の核は、形状を事前に知っている前提で距離を直接評価し、その距離と pixel size から alpha を決める点にある
- pixel size の求め方として `fwidth()` と `length(vec2(dFdx(dist), dFdy(dist)))` の差が議論され、`fwidth()` は対角方向の近似誤差で菱形っぽい歪みを生みうるとされる
- 2D では per-object に pixel size を事前計算して uniform 的に渡す手もあり、これにより per-pixel derivative 計算を避けられる場合がある
- blending については `smoothstep()` より線形補間、さらには単純除算まで簡約できるという主張がある
- 境界を quad の外へはみ出させないための shrinking / breathing room、MSAA + alpha-to-coverage での edge case、複数 shape を 1 quad で描く場合の clamp/weighted sum の必要性も扱う

## このリポジトリへの対応

- `doc/memo/anti_aliasing.md` の `smoothstep` / `fwidth` ベース実装は、この AAA 文脈の一変種として理解できる
- `overlap_shader.wgsl` の `linerstep` 導入や `fwidth` の分岐前評価は、記事中の「線形補間」「derivative の扱い」の論点と親和性が高い
- thin triangle や conservative rasterization、境界外の未ラスタライズ問題は、この repo の debug shader / artifact メモとも強く接続する

## 現行実装との違い

- 記事は円や単純 shape の SDF から AAA を説明するが、この repo は複数の直線・二次ベジェ三角形の積算と resolve で文字輪郭を扱う
- 記事は `smoothstep` を批判的に扱うが、repo の memo と一部 shader では `smoothstep` / `linerstep` の両方が使われている
- 記事は `fwidth()` の近似誤差を問題視するが、repo 側は速度と実装容易性のバランスで `fwidth` ベースを採る場面がある

## 注目キーワード

- analytical anti-aliasing
- signed distance field
- one-pixel fade
- `fwidth()`
- `dFdx` / `dFdy`
- linearstep
- alpha to coverage
- conservative rasterization