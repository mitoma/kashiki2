---
title: Glyph Model
kind: component
status: draft
updated: 2026-07-19
source_refs:
  - ../../memo/font.md
related_pages:
  - ../concepts/text-rendering-pipeline.md
  - ../sources/source-font-glyph-model.md
---

# Glyph Model

## 概要

グリフ管理の最小モデルは、char から glyph 群へ到達し、各 glyph が id / direction / width を持つ形で整理されている。

## 主要要素

- char
  - どの文字に対応するか
  - 1 文字が複数 glyph 候補を持ちうる入口
- glyph
  - glyph id
  - direction
  - width
- direction
  - horizontal
  - vertical
  - both
- width
  - regular
  - wide

## このモデルが効く箇所

- 縦書き / 横書き切り替え
- 幅計算と折り返し
- glyph 選択とインスタンス生成

## 未整理の論点

- 実コードでの型配置と API の対応
- char から複数 glyph 候補を解決する規則