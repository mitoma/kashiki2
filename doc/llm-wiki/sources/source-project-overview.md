---
title: Source Summary - Project Overview
kind: source
status: production
updated: 2026-07-19
source_refs:
  - ../../project.md
related_pages:
  - ../concepts/text-rendering-pipeline.md
---

# Source Summary - Project Overview

## 対象 source

- `doc/project.md`

## 要約

- 炊紙は Rust と WebGPU を用いた 3D 空間テキストエディタである
- ワークスペースは機能ごとにクレート分割されている
- `font_rasterizer` が GPU フォント描画、`text_buffer` がテキスト編集とレイアウト、`highlighter` が構文解析、`stroke_parser` が入力解析を担う

## wiki への影響

- クレート間の責務分担を concepts / components の骨格に使う
- 特に `font_rasterizer` と `text_buffer` を初期優先とする