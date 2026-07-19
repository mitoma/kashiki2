---
title: World Scene Architecture
kind: concept
status: draft
updated: 2026-07-19
source_refs:
  - ../../kashikishi/src/world/mod.rs
  - ../../kashikishi/src/world/start_world.rs
  - ../../kashikishi/src/world/categorized_memos_world.rs
  - ../../kashikishi/src/world/markdown_presentation_world.rs
  - ../../memo/layout_memo.md
related_pages:
  - ../sources/source-modal-worlds.md
  - ../concepts/layout-engine.md
  - ../concepts/action-system.md
---

# World Scene Architecture

## 概要

`kashikishi` は `ModalWorld` を境界として、複数の scene 相当の world を切り替える構成を取る。各 world は実体として `ui_support::layout_engine::World` を保持し、`UiContext` と `Action` を介して振る舞う。

## 主要要素

- `ModalWorld`
  - `get` / `get_mut` で内部 `World` を公開する
  - `apply_action` で world 固有コマンドを処理する
  - `world_chars` で必要 glyph 集合を返す
  - `add_modal` で modal UI の追加方法を定義する
- `StartWorld`
  - `StackLayout` 上にロゴとモード選択を置く入口 world
- `CategorizedMemosWorld`
  - memo モデルと `DefaultWorld` を同期しながら TextEdit 群を再構築する編集 world
- `MarkdownPresentationWorld`
  - FileChooser から markdown を開き、見出し分割した TextEdit 群へ変換する world

## 重要な構造

- 実際の mode 切り替えは `main.rs` の callback 側が担当し、`mode` コマンドで world 実装を差し替える
- 各 world は `UiContext` から direction、window size、glyph 登録、post action などを使う
- modal の見せ方や camera adjustment は world ごとに選べるが、`DefaultWorld` ベースのレイアウト更新を共有する

## 未整理の論点

- `HelpWorld` を含む全 world の共通規約を別ページに切り出すか
- layout memo の `Paper` / `Text` / `World` の概念と現行実装の差分整理