---
title: Text Rendering Pipeline
kind: concept
status: draft
updated: 2026-07-19
source_refs:
  - ../../project.md
  - ../../memo/anti_aliasing.md
  - ../../memo/font.md
  - ../../font_rasterizer/src/rasterizer_pipeline.rs
  - ../../font_rasterizer/src/vector_vertex.rs
  - ../../font_rasterizer/src/shader/overlap_shader.wgsl
  - ../../font_rasterizer/src/shader/outline_shader.wgsl
related_pages:
  - ../decisions/anti-aliasing-strategy.md
  - ../components/glyph-model.md
  - ../components/vector-vertex-builder.md
  - ../sources/source-project-overview.md
  - ../sources/source-anti-aliasing.md
  - ../sources/source-font-glyph-model.md
  - ../sources/source-rasterizer-pipeline.md
  - ../sources/source-vector-vertex-builder.md
  - ../sources/source-overlap-shader.md
  - ../sources/source-outline-shader.md
---

# Text Rendering Pipeline

## 概要

炊紙の GPU テキスト描画は、フォント形状のベクトル情報を GPU に渡し、複数段のシェーダ処理で塗りとアンチエイリアシングを解決する構造を持つ。

## このページで扱う範囲

- `font_rasterizer` が担当する描画パイプラインの全体像
- ベクトル頂点、overlap、resolve の責務分担
- AA 戦略や fill rule と結び付く論点
- char から glyph、direction、width へ落ちる最小データモデル

## 現時点の要点

- `font_rasterizer` は GPU ベースのフォント描画を担当する中核クレートである
- AA は `smoothstep` と `fwidth` を使う analytical anti-aliasing を採用している
- グリフには char 対応、glyph id、direction、width の軸があり、縦横レイアウトや幅解決と接続する
- `RasterizerPipeline` は overlap/outline を実行する `RasterizerRenderrer` と、screen/background/shader-art の最終表示段を束ねる
- `Quarity` は oversampling と GPU 上限考慮を持つ解像度ポリシーである
- `VectorVertexBuilder` は `OutlineBuilder` 実装として move/line/quad/cubic/close を GPU 向け三角形列へ変換する
- `overlap_shader.wgsl` は vertex_type を wait / triangle_type に写像し、multi render target へ winding と edge 情報を積算する
- `outline_shader.wgsl` は overlap count と outline texture を resolve し、even-odd / non-zero の fill rule ごとに最終 alpha を決める

## 未整理の論点

- vector vertex 生成から shader resolve までのデータ流れの明文化
- debug shader と production shader の運用差分