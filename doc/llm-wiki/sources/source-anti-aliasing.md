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
  - ./source-frost-analytical-anti-aliasing.md
---

# Source Summary - Anti Aliasing

## 対象 source

- `doc/memo/anti_aliasing.md`

## 要約

- 炊紙は analytical anti-aliasing を採用している
- ベジェ曲線、ベジェ補助直線、直線を頂点タイプで区別し、それぞれの距離計算を shader で行う
- `smoothstep` と `fwidth` が主要なエッジ解決手段である
- ただし外部記事側には `smoothstep` より線形補間や単純除算を推す議論もあり、現行実装との差分として見ておく価値がある

## wiki への影響

- text rendering pipeline と AA strategy の出発点になる
- 追加で shader 実装と historical note を source 化して補強する必要がある
- 外部記事を加えることで、`fwidth`、1px フェード、SDF ベース境界処理の設計意図を比較できる