---
title: Unicode Segmentation
kind: decision
status: production
updated: 2026-07-19
source_refs:
  - ../../text_buffer_wrapping.md
related_pages:
  - ../concepts/layout-engine.md
  - ../sources/source-text-buffer-wrapping.md
---

# Unicode Segmentation

## 決定

`text_buffer` の改行機会取得には ICU4X の `icu_segmenter` を採用する方針とする。

## 理由

- 必要要件が Unicode line breaking opportunity の取得にある
- 軽量なライブラリとして依存コストを抑えたい
- 既存の禁則・インデント規則と段階的に統合しやすい

## 影響範囲

- `PhysicalLayoutCalculator` の折り返し判定
- 幅超過時の分岐順序
- preedit を含むレイアウト経路の共通化

## 未確定点

- 実装の完了状態とテスト範囲はコード source 化後に見直す