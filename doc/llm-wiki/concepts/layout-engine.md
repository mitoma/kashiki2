---
title: Layout Engine
kind: concept
status: draft
updated: 2026-07-19
source_refs:
  - ../../text_buffer_wrapping.md
  - ../../../memories/repo/text_buffer_design.md
  - ../../text_buffer/src/layout.rs
related_pages:
  - ../decisions/unicode-segmentation.md
  - ../decisions/preedit-source-of-truth.md
  - ../sources/source-text-buffer-wrapping.md
  - ../sources/source-text-buffer-design.md
  - ../sources/source-text-buffer-layout-code.md
  - ../concepts/editor-buffer-model.md
---

# Layout Engine

## 概要

炊紙のレイアウトは `text_buffer` を中心に、文字幅、禁則、改行機会、preedit 配置を統合して physical layout を計算する。

## 現時点の要点

- 折り返しは単純な幅超過だけでなく、Unicode の改行機会を取り入れる方向で整理されている
- 既存の禁則、継続インデント、preedit 表示を崩さずに line breaking を拡張する必要がある
- preedit の描画位置は UI 側で再計算せず、`text_buffer` 側の physical layout を唯一の正とする方針がある
- `text_buffer` の公開互換面と `ChangeEvent` の順序依存は、UI 側挙動を壊さない前提として維持されるべきである
- `PhysicalLayoutCalculator` は break opportunity の収集、backtrack wrap、speaker/list indent、preedit 注入を 1 本の計算経路に統合している

## 未整理の論点

- `editor.rs` と `layout.rs` の責務境界
- 幅解決と折り返し判定のテスト観点
- logical position と physical position の関係整理