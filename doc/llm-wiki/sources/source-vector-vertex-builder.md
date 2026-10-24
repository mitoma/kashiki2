---
title: Source Summary - Vector Vertex Builder
kind: source
status: production
updated: 2026-10-11
source_refs:
  - ../../../font_rasterizer/src/vector_vertex.rs
  - ../../../font_rasterizer/src/windfoil.rs
  - ../../../font_rasterizer/src/glyph_cache.rs
  - ../../../font_rasterizer/src/straighten_outline_builder.rs
  - ../../../font_rasterizer/src/straight_run_simplifier.rs
  - ../../memo/E_midline_geometry_structure.md
related_pages:
  - ../components/vector-vertex-builder.md
  - ../concepts/text-rendering-pipeline.md
  - ../decisions/anti-aliasing-strategy.md
---

# Source Summary - Vector Vertex Builder

## 対象 source

- `font_rasterizer/src/vector_vertex.rs`
- `font_rasterizer/src/windfoil.rs`
- `font_rasterizer/src/glyph_cache.rs`

## 要約

- `VectorVertexBuilder` は skrifa の `OutlinePen` を実装し、アウトライン命令列を有向二次曲線列へ変換する
- `QuadraticCurve` は `start` / `control` / `end` の三つの座標を持つ。直線は制御点が中点の二次曲線であり、区間種別のタグは持たない
- `close` は開始点へ戻る区間だけを追加する。補助三角形・重心・予約インデックスは作らない
- `build` は三つの座標へ従来の座標系・中心・em・スケール変換を適用する
- Windfoil はこの型を直接使用し、単調分割した曲線と行バンドを storage buffer に格納する。旧三角形からの曲線再抽出は不要
- キャッシュは版番号付きファイルと payload を使用し、横書き・縦書きの曲線列を little-endian で保存する。旧頂点形式は誤読せず再生成する
- `StraightenOutlineBuilder` は入力をサブパス単位で保持し、曲率判定と共線判定により連続するほぼ直線区間をまとめる
- `VectorVertexBuilder::quad_to` でも単独のほぼ直線曲線を `line_to` へ変換する
- デバッグ画像も曲線と三つの座標を表示する。`VertexPointKind` と三角形デバッグ API は廃止した

## 過去の方式

`doc/memo/E_midline_geometry_structure.md` の頂点タイプ・補間ウエイト・扇形三角形の説明は旧方式の記録であり、現行の生成・描画には使用しない。

## wiki への影響

- builder component と描画 pipeline の基本単位を、有向二次曲線として統一する
- AA strategy では曲線積分と巻き数折り畳みの制約を扱い、旧三角形方式の対策とは分ける