---
title: Layout Engine
kind: concept
status: draft
updated: 2026-07-19
source_refs:
  - ../../text_buffer_wrapping.md
related_pages:
  - ../decisions/unicode-segmentation.md
  - ../decisions/preedit-source-of-truth.md
  - ../sources/source-text-buffer-wrapping.md
---

# Layout Engine

## 概要

炊紙のレイアウトは `text_buffer` を中心に、文字幅、禁則、改行機会、preedit 配置を統合して physical layout を計算する。

## 現時点の要点

- 折り返しは単純な幅超過だけでなく、Unicode の改行機会を取り入れる方向で整理されている
- 既存の禁則、継続インデント、preedit 表示を崩さずに line breaking を拡張する必要がある
- preedit の描画位置は UI 側で再計算せず、`text_buffer` 側の physical layout を唯一の正とする方針がある

## 未整理の論点

- `editor.rs` と `layout.rs` の責務境界
- 幅解決と折り返し判定のテスト観点
- logical position と physical position の関係整理