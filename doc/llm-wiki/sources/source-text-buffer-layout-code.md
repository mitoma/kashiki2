---
title: Source Summary - Text Buffer Layout Code
kind: source
status: production
updated: 2026-07-19
source_refs:
  - ../../text_buffer/src/layout.rs
related_pages:
  - ../concepts/layout-engine.md
  - ../concepts/editor-buffer-model.md
---

# Source Summary - Text Buffer Layout Code

## 対象 source

- `text_buffer/src/layout.rs`

## 要約

- `PhysicalLayout` は描画用文字位置、preedit 位置、main caret 位置、mark 位置を保持する
- `PhysicalLayoutCalculator` は buffer 全行を走査し、折り返し、禁則、caret/mark、preedit を 1 本の経路で計算する
- `try_backtrack_wrap` は break candidate を使った後戻り折り返しを行い、既に配置済みの chars / preedit / caret / mark を次行へ移し替える
- `calc_indent` は list pattern と `名前: ` 形式の speaker indent を検出する
- `apply_line_break_rules` は Unicode 改行機会、行頭禁則、行末禁則、幅超過を統合して改行判定を行う
- `insert_preedit_chars` は preedit を通常の layout 規則に沿って挿入し、logical position と physical position を両方追跡する

## wiki への影響

- layout engine の実装核を source として固定できる
- preedit source of truth と unicode segmentation decision の実装面を補強できる