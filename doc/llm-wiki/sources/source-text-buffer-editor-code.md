---
title: Source Summary - Text Buffer Editor Code
kind: source
status: production
updated: 2026-07-19
source_refs:
  - ../../text_buffer/src/editor.rs
related_pages:
  - ../concepts/editor-buffer-model.md
  - ../concepts/layout-engine.md
---

# Source Summary - Text Buffer Editor Code

## 対象 source

- `text_buffer/src/editor.rs`

## 要約

- `Editor` は main caret, mark, buffer, undo list, `ChangeEvent` sender を所有する
- `operation` は mark / unmark / undo を特別扱いし、それ以外は `BufferApplyer::apply_action` に委譲する
- `action_width_selection_update` は操作前後の selection 差分を比較し、必要な `SelectChar` / `UnSelectChar` を送る
- unmark 対象操作では buffer 変更前に解除通知を送る必要がある
- `calc_phisical_layout` は `PhysicalLayoutCalculator` を生成して layout 計算を委譲する
- `ChangeEvent` は char と caret の add / move / remove と selection の変化を表す

## wiki への影響

- editor buffer model の source になる
- UI 側が依存する変更通知順の制約を追跡できる