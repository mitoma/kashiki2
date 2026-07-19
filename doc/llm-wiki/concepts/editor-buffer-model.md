---
title: Editor Buffer Model
kind: concept
status: draft
updated: 2026-07-19
source_refs:
  - ../../text_buffer/src/editor.rs
  - ../../../memories/repo/text_buffer_design.md
related_pages:
  - ../concepts/layout-engine.md
  - ../concepts/action-system.md
  - ../sources/source-text-buffer-editor-code.md
  - ../sources/source-text-buffer-design.md
---

# Editor Buffer Model

## 概要

`text_buffer::Editor` は `Buffer`, main caret, mark, undo list, `ChangeEvent` sender をまとめる編集の中核モデルである。

## 主要責務

- `operation` で `EditorOperation` を受け取り、buffer と caret を更新する
- selection の出入りを監視し、`SelectChar` / `UnSelectChar` を通知する
- `calc_phisical_layout` で layout 計算を `PhysicalLayoutCalculator` に委譲する
- undo 用に `ReverseActions` を保持する

## 重要な制約

- `ChangeEvent` の通知順は UI 側が依存しているため崩せない
- unmark 対象の操作では、buffer 変更後ではなく前後の選択差分を意識した通知が必要になる
- preedit は `Editor` 自身で再計算せず layout 側の結果を利用する

## 未整理の論点

- `BufferApplyer` と `Editor` の責務境界
- sender 切断時の無視方針がコード上でどこまで徹底されているか