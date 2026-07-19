---
title: Action System
kind: concept
status: draft
updated: 2026-07-19
source_refs:
  - ../../text_buffer/src/action.rs
related_pages:
  - ../concepts/editor-buffer-model.md
  - ../sources/source-text-buffer-action-code.md
---

# Action System

## 概要

`text_buffer` の action system は `EditorOperation` を単位として編集、移動、選択、コピー、検索系操作を表現し、`ReverseAction` と `BufferApplyer` で undo を構成する。

## 主要要素

- `EditorOperation`
  - caret 移動
  - 文字列編集
  - undo / mark / unmark
  - copy / cut
  - highlight / キーワード移動
- `ReverseAction` / `ReverseActions`
  - undo 用の逆操作列
- `BufferApplyer`
  - `EditorOperation` を buffer / caret / mark に適用する実行系

## 重要なルール

- `is_unmark_operation` は、buffer 変更で mark 位置が壊れる操作をまとめて判定する
- `is_single_line_operation` は UI や補助処理が依存できる軽量な分類を提供する
- undo は逆操作列の再適用で構成される

## 未整理の論点

- `Highlight` / `MoveToNext` / `MoveToPrevious` の上位利用箇所との接続
- 操作分類が UI にどう公開されているか