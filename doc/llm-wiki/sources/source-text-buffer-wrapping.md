---
title: Source Summary - Text Buffer Wrapping
kind: source
status: production
updated: 2026-07-19
source_refs:
  - ../../text_buffer_wrapping.md
related_pages:
  - ../concepts/layout-engine.md
  - ../decisions/unicode-segmentation.md
  - ../decisions/preedit-source-of-truth.md
---

# Source Summary - Text Buffer Wrapping

## 対象 source

- `doc/text_buffer_wrapping.md`

## 要約

- 既存の折り返しは表示幅超過と独自禁則に偏っていた
- Unicode UAX #14 ベースの改行機会を ICU4X `icu_segmenter` で取得する方針が示されている
- 既存禁則、継続インデント、preedit 表示は維持しながら統合する

## wiki への影響

- layout engine 概念ページの中核 source になる
- unicode segmentation の decision を支える
- preedit を通常レイアウト経路と共通化する判断にも接続する