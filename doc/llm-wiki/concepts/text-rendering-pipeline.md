---
title: Text Rendering Pipeline
kind: concept
status: draft
updated: 2026-07-19
source_refs:
  - ../../project.md
  - ../../memo/anti_aliasing.md
related_pages:
  - ../decisions/anti-aliasing-strategy.md
  - ../sources/source-project-overview.md
  - ../sources/source-anti-aliasing.md
---

# Text Rendering Pipeline

## 概要

炊紙の GPU テキスト描画は、フォント形状のベクトル情報を GPU に渡し、複数段のシェーダ処理で塗りとアンチエイリアシングを解決する構造を持つ。

## このページで扱う範囲

- `font_rasterizer` が担当する描画パイプラインの全体像
- ベクトル頂点、overlap、resolve の責務分担
- AA 戦略や fill rule と結び付く論点

## 現時点の要点

- `font_rasterizer` は GPU ベースのフォント描画を担当する中核クレートである
- AA は `smoothstep` と `fwidth` を使う analytical anti-aliasing を採用している
- detailed な実装責務は今後 `rasterizer_pipeline.rs`, `vector_vertex.rs`, `overlap_shader.wgsl`, `outline_shader.wgsl` を source 化して補強する

## 未整理の論点

- vector vertex 生成から shader resolve までのデータ流れの明文化
- non-zero / even-odd の差分位置
- debug shader と production shader の運用差分