---
title: Source Summary - Text Buffer Action Code
kind: source
status: production
updated: 2026-07-19
source_refs:
  - ../../text_buffer/src/action.rs
related_pages:
  - ../concepts/action-system.md
  - ../concepts/editor-buffer-model.md
---

# Source Summary - Text Buffer Action Code

## 対象 source

- `text_buffer/src/action.rs`

## 要約

- `EditorOperation` は移動、編集、選択、undo、copy/cut、highlight 系をまとめる操作 enum である
- `is_unmark_operation` は mark を維持しない操作群を分類する
- `is_single_line_operation` は単一行前提の分類を提供するが、`InsertString` だけは改行を含みうる例外として注意が必要である
- `ReverseAction` と `ReverseActions` は undo 用の逆操作列を表す
- `BufferApplyer` は各 `EditorOperation` を buffer / caret / mark に適用し、対応する逆操作列を返す

## wiki への影響

- action system 概念の基礎 source になる
- undo の構造と操作分類を code source として追跡できる