---
title: Source Summary - Text Buffer Design
kind: source
status: production
updated: 2026-07-19
source_refs:
  - ../../../memories/repo/text_buffer_design.md
related_pages:
  - ../concepts/layout-engine.md
  - ../decisions/preedit-source-of-truth.md
  - ../decisions/unicode-segmentation.md
---

# Source Summary - Text Buffer Design

## 対象 source

- `/memories/repo/text_buffer_design.md`

## 要約

- `text_buffer` の公開互換面は `Editor::new`, `operation`, `to_buffer_string`, `buffer_chars`, `calc_phisical_layout` などを優先維持する
- `ChangeEvent` は UI 側で順序依存があり、特に `MoveChar` の連続順序と選択解除通知を崩してはいけない
- sender 切断時は panic ではなく継続を優先する
- preedit 配置は `PhisicalLayout.preedit_chars` を唯一の正とする
- 折り返しは ICU4X の `icu_segmenter` を既存禁則・インデント規則と合成する方針である

## wiki への影響

- layout engine の責務と互換性制約を明示できる
- preedit source of truth と unicode segmentation の decision を補強できる