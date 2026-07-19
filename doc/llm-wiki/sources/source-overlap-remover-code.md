---
title: Source Summary - Overlap Remover Code
kind: source
status: production
updated: 2026-07-19
source_refs:
  - ../../ttf_overlap_remover/src/lib.rs
  - ../../ttf_overlap_remover/src/path_segment.rs
related_pages:
  - ../components/overlap-remover.md
  - ../decisions/overlap-removal-strategy.md
  - ../concepts/text-rendering-pipeline.md
---

# Source Summary - Overlap Remover Code

## 対象 source

- `ttf_overlap_remover/src/lib.rs`
- `ttf_overlap_remover/src/path_segment.rs`

## 要約

- crate は TTF グリフの複数 subpath から重複領域を除去し、even-odd でも元の non-zero 見た目を再現することを目的とする
- `PathSegment` は line / quadratic / cubic を統一的に扱い、evaluate / flatten / vector / bounding rect を提供する
- 主処理は `remove_path_overlap` で、subpath 分解、交差分割、boundary filter、loop / chain 再構成、Path への戻しを行う
- `build_chains` は交差精度のギャップを広い許容値でつなぐ救済処理である
- テストは絵文字や交差図形で Winding と EvenOdd の一致率を確認している

## wiki への影響

- overlap remover の algorithm と存在理由を component / decision に昇格できる
- non-zero 経路との役割重複を議論する基礎になる