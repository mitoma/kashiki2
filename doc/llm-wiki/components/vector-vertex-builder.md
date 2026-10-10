---
title: Vector Vertex Builder
kind: component
status: production
updated: 2026-10-11
source_refs:
  - ../../../font_rasterizer/src/vector_vertex.rs
  - ../../../font_rasterizer/src/windfoil.rs
  - ../../../font_rasterizer/src/glyph_cache.rs
  - ../../memo/E_midline_geometry_structure.md
related_pages:
  - ../concepts/text-rendering-pipeline.md
  - ../sources/source-vector-vertex-builder.md
---

# Vector Vertex Builder

## 概要

`VectorVertexBuilder` は TTF / SVG のアウトライン入力を受け、有向二次曲線列を持つ `VectorVertex` へ変換する `OutlinePen` 実装である。型名は既存呼び出し側のために維持しているが、頂点・三角形インデックスのモデルではない。

## 主要責務

- `move_to`, `line_to`, `quad_to`, `curve_to`, `close` を通じて有向の `QuadraticCurve { start, control, end }` を構成する
- 直線の制御点は始点・終点の中点とし、直線と二次曲線を同じ構造で保持する
- close 時は輪郭開始点への区間のみを追加し、巻き方向を保つ
- 補助三角形・中心点探索・頂点種別は生成しない
- `quad_to` では曲率サンプリングでほぼ直線と判定された曲線を `line_to` に簡約する
- `curves()` で形状を参照でき、デバッグ表示も始点・制御点・終点を直接使用する

## 重要な実装上の点

- Builder の状態は曲線列、現在点、輪郭開始点、変換オプションだけである
- Windfoil は同じ型を単調分割・行バンド分類に使用し、曲線を再抽出しない
- キャッシュは `glyph_cache_v2_*.redb` に曲線列を保存する。旧版とはファイル名・payload version・redb value type を分離する
- `CoordinateSystem` と `VertexBuilderOptions` により SVG / Font 座標系やスケール変換を切り替える
- `StraightenOutlineBuilder` によるサブパス全体の直線ラン簡約を経由することで、単発ではなく連続するほぼ直線の区間も 1 本へまとめられる

## 関連する品質上の注意

- 曲率判定は 20 点のサンプリングと固定しきい値を使うため、簡約は「実質的に直線」の入力を対象とする
- `curve_to` は先に 3 次から 2 次へ近似分解され、その後に直線簡約の対象となる

## 過去の方式

扇形三角形、Bezier fill 専用頂点、`FlipFlop`、重心探索は旧 rasterizer のための表現だった。Windfoil への移行により、これらを保持する必要がなくなった。