---
title: 構文解析とシンタックスハイライト
kind: component
status: production
updated: 2026-08-14
source_refs:
  - ../../highlighter/src/lib.rs
  - ../../markdown_heading_splitter/src/lib.rs
  - ../../Cargo.toml
related_pages:
  - ../sources/source-syntax-analysis-code.md
  - ../concepts/editor-buffer-model.md
---

# 構文解析とシンタックスハイライト

## 現行構成

構文解析の共通基盤には Arborium fork を使う。Arborium は tree-sitter 互換の parser API と、feature で選択した各言語の grammar module をまとめて提供する。したがって、アプリケーションコードが直接依存する構文解析面は `arborium::tree_sitter` と `arborium::lang_*` で統一されている。

## 解析経路

- `highlighter`: Markdown を解析し、inline 部分と code fence の内容を対応する grammar で入れ子に解析する。
- `markdown_heading_splitter`: Markdown の構文木から ATX / Setext 見出しを収集し、見出し単位の本文を返す。
- 言語選択: code fence の info string を読み、Rust、Java、Go、JSON、Bash、TOML の parser を選択する。
- 範囲表現: Arborium の byte range を既存 API の文字 range に変換する。

## 移行上の判断

今回の移行は構文木の利用方法を変えるものではなく、grammar の配布、feature 選択、Markdown inline grammar の利用を Arborium に集約するもの。依存の直接指定を個別に増やさず、workspace の Arborium 設定を source of truth とする。

## 検証

構文解析コードを変更した場合は、対象 crate のテストに加えて `mise r check` を実行する。特に UTF-8 の文字範囲、Markdown inline、各 code fence 言語、ATX / Setext 見出しを確認する。
