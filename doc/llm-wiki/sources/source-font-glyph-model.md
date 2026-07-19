---
title: Source Summary - Font Glyph Model
kind: source
status: production
updated: 2026-07-19
source_refs:
  - ../../memo/font.md
related_pages:
  - ../components/glyph-model.md
  - ../concepts/text-rendering-pipeline.md
---

# Source Summary - Font Glyph Model

## 対象 source

- `doc/memo/font.md`

## 要約

- グリフには char 対応、glyph id、direction、width の属性がある
- direction は horizontal / vertical / both を想定している
- width は regular / wide を想定している
- char から複数 glyph への関連を持つ最小モデルとして整理されている

## wiki への影響

- glyph モデルの component を定義する入口になる
- 縦横方向と幅の扱いを text rendering pipeline の前提知識として接続できる