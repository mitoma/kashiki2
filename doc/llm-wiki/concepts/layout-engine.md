---
title: Layout Engine
kind: concept
status: draft
updated: 2026-07-19
source_refs:
  - ../../text_buffer_wrapping.md
  - ../../../memories/repo/text_buffer_design.md
  - ../../../phisical_layouter/src/lib.rs
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

炊紙のレイアウトは、編集状態を保持する `text_buffer` と、physical layout を計算する `phisical_layouter` を分離して構成される。

## 現時点の要点

- 折り返しは単純な幅超過だけでなく、Unicode の改行機会を取り入れる方向で整理されている
- 既存の禁則、継続インデント、preedit 表示を崩さずに line breaking を拡張する必要がある
- preedit の描画位置は UI 側で再計算せず、`phisical_layouter` が返す `preedit_chars` を唯一の正とする方針がある
- `text_buffer` の公開互換面と `ChangeEvent` の順序依存は、UI 側挙動を壊さない前提として維持されるべきである
- `PhysicalLayoutCalculator` は break opportunity の収集、backtrack wrap、speaker/list indent、preedit 注入を 1 本の計算経路に統合している

## 未整理の論点

- `text_buffer` と `phisical_layouter` の API 境界をどこまで安定化するか
- 幅解決と折り返し判定のテスト観点
- logical position と physical position の関係整理