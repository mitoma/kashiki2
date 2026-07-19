---
title: Source Summary - Modal Worlds
kind: source
status: production
updated: 2026-07-19
source_refs:
  - ../../kashikishi/src/world/mod.rs
  - ../../kashikishi/src/world/start_world.rs
  - ../../kashikishi/src/world/categorized_memos_world.rs
  - ../../kashikishi/src/world/markdown_presentation_world.rs
  - ../../kashikishi/src/main.rs
  - ../../memo/layout_memo.md
related_pages:
  - ../concepts/world-scene-architecture.md
  - ../concepts/layout-engine.md
---

# Source Summary - Modal Worlds

## 対象 source

- `kashikishi/src/world/mod.rs`
- `kashikishi/src/world/start_world.rs`
- `kashikishi/src/world/categorized_memos_world.rs`
- `kashikishi/src/world/markdown_presentation_world.rs`
- `kashikishi/src/main.rs`
- `doc/memo/layout_memo.md`

## 要約

- `ModalWorld` は `World` 実装のラッパー兼 scene 切り替え境界で、action 処理、glyph 集合、modal 追加、終了時処理を定義する
- `main.rs` の callback は `mode` コマンドで `StartWorld`, `CategorizedMemosWorld`, `MarkdownPresentationWorld`, `HelpWorld` へ切り替える
- `StartWorld` は StackLayout 上の選択メニューで mode 入口を提供する
- `CategorizedMemosWorld` は memo モデルを `DefaultWorld` 上の複数 `TextEdit` に再構成し、保存時に world から memo へ逆変換する
- `MarkdownPresentationWorld` は FileChooser と heading splitter を使って markdown をスライド状の TextEdit 群へ変換する
- `layout_memo.md` は World / Paper / Text の抽象概念を示すが、現行実装は `DefaultWorld` と UI component 群がその責務を分担している

## wiki への影響

- World / scene 構成を concepts として整理する基礎になる
- 将来 `HelpWorld` や modal パターンの追加説明を広げる入口になる