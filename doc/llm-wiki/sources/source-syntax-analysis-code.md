---
title: Arborium 構文解析コード
kind: source
status: production
updated: 2026-08-14
source_refs:
  - ../../highlighter/src/lib.rs
  - ../../markdown_heading_splitter/src/lib.rs
  - ../../Cargo.toml
related_pages:
  - ../components/syntax-analysis.md
  - ../sources/source-project-overview.md
---

# Arborium 構文解析コード

## 要約

構文解析の依存関係は `tree-sitter` の個別 grammar から、`arborium` fork の workspace 依存へ移行している。アプリケーション側は `arborium::tree_sitter` の parser / node / cursor API を使い、言語ごとの grammar は `arborium::lang_*::language()` から取得する。

## highlighter

`highlighter` は Markdown を親 grammar として解析する。`inline` ノードは `lang_markdown_inline` で再解析し、Markdown の code fence 内は info string で判定した言語の parser で再解析する。現行の code fence 対応は Rust、Java、Go、JSON、Bash、TOML。解析結果はノードの kind と文字範囲を `HighlightSettings` に渡し、設定済みの highlight category へ変換する。

バイト位置で返る構文木の範囲は、既存の highlight API が文字位置を要求するため、UTF-8 の byte offset から文字位置へ変換している。

## markdown_heading_splitter

`markdown_heading_splitter` は `arborium::lang_markdown` で文書を一度解析し、`atx_heading` と `setext_heading` を再帰的に収集する。各見出しの終了位置から次の見出しの開始位置までを本文として切り出し、前後の空白を trim して `Vec<(Heading, String)>` を返す。H1〜H6 と Setext の H1 / H2 を扱う。

## 注意点

- Arborium の依存は現時点で `support_markdown_inline_dist` branch の fork を参照している。
- `default-features = false` とし、Markdown、Markdown inline、Rust、Java、Go、JSON、Bash、TOML の language feature を明示している。
- Arborium 内部には tree-sitter 系 crate が残るが、workspace の直接利用者は Arborium の re-export と language module を使う。
