---
title: Source Summary - Anti Aliasing
kind: source
status: production
updated: 2026-07-19
source_refs:
  - ../../memo/anti_aliasing.md
related_pages:
  - ../concepts/text-rendering-pipeline.md
  - ../decisions/anti-aliasing-strategy.md
---

# Source Summary - Anti Aliasing

## 対象 source

- `doc/memo/anti_aliasing.md`

## 要約

- 炊紙は analytical anti-aliasing を採用している
- ベジェ曲線、ベジェ補助直線、直線を頂点タイプで区別し、それぞれの距離計算を shader で行う
- `smoothstep` と `fwidth` が主要なエッジ解決手段である

## wiki への影響

- text rendering pipeline と AA strategy の出発点になる
- 追加で shader 実装と historical note を source 化して補強する必要がある