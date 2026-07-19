---
title: Preedit Source Of Truth
kind: decision
status: production
updated: 2026-07-19
source_refs:
  - ../../../memories/repo/text_buffer_design.md
  - ../../../memories/repo/preedit_layout_fix.md
related_pages:
  - ../concepts/layout-engine.md
  - ../sources/source-text-buffer-design.md
---

# Preedit Source Of Truth

## 決定

IME preedit の描画位置は `text_buffer` 側の physical layout、特に `preedit_chars` を唯一の正とする。

## 理由

- UI 側の独自再計算は折り返し、禁則、オフセットのずれを招きやすい
- preedit は通常文字列と同じレイアウト規則に従うべきである
- 過去の不具合修正でも、改行直後の line head 判定や logical position 更新が重要だった
- `text_buffer` の公開互換面と `ChangeEvent` 順序依存を崩さないことが、UI 側の修正安全性に直結する

## 運用上の含意

- preedit 表示変更時は UI だけでなく `text_buffer` の layout を先に確認する
- 関連修正では `editor.rs` と layout 計算結果の整合を確認する