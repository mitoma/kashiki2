---
title: URL Source Ingest
kind: workflow
status: production
updated: 2026-07-19
source_refs:
  - ../schema.md
  - ./ingest-workflow.md
related_pages:
  - ../sources/source-evan-wallace-gpu-text-rendering.md
  - ../index.md
---

# URL Source Ingest

## 目的

外部の Web 記事や論文ページを、raw 本文を repo に置かずに知識ベースへ取り込む。

## 手順

1. 原文 URL とメタデータを確認する
2. public repo への転載可否を確認する
3. `sources/` に要約ページを作り、`source_url` と `retrieved_at` を記録する
4. `raw_capture: none` を設定し、本文転載をしていないことを明示する
5. 関連 concepts / components / decisions を更新する
6. `index.md` と `log.md` を更新する

## 要約ページに含める項目

- source の目的
- repo に関係する主張
- 実装と結び付くキーワード
- 現行コードとの差分や未解決点

## 避けること

- 記事本文の全文転載
- 図版や画像の再配布
- 出典 URL なしの二次要約