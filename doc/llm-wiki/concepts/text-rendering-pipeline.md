---
title: Text Rendering Pipeline
kind: concept
status: production
updated: 2026-10-11
source_refs:
  - ../../project.md
  - ../../memo/anti_aliasing.md
  - ../../memo/font.md
  - ../../../font_rasterizer/src/rasterizer_pipeline.rs
  - ../../../font_rasterizer/src/rasterizer_renderrer.rs
  - ../../../font_rasterizer/src/vector_vertex.rs
  - ../../../font_rasterizer/src/windfoil.rs
  - ../../../font_rasterizer/src/glyph_cache.rs
  - ../../../font_rasterizer/src/shader/windfoil.wgsl
  - https://medium.com/@evanwallace/easy-scalable-text-rendering-on-the-gpu-c3f4d782c5ac
related_pages:
  - ../decisions/anti-aliasing-strategy.md
  - ../decisions/overlap-removal-strategy.md
  - ../components/glyph-model.md
  - ../components/vector-vertex-builder.md
  - ../components/overlap-remover.md
  - ../components/shader-art-system.md
  - ../sources/source-project-overview.md
  - ../sources/source-anti-aliasing.md
  - ../sources/source-font-glyph-model.md
  - ../sources/source-rasterizer-pipeline.md
  - ../sources/source-vector-vertex-builder.md
  - ../sources/source-overlap-shader.md
  - ../sources/source-outline-shader.md
  - ../sources/source-overlap-remover-code.md
  - ../sources/source-shader-art-system.md
  - ../sources/source-evan-wallace-gpu-text-rendering.md
---

# Text Rendering Pipeline

## 概要

炊紙の GPU テキスト描画は、フォント形状のベクトル情報を GPU に渡し、複数段のシェーダ処理で塗りとアンチエイリアシングを解決する構造を持つ。

## このページで扱う範囲

- `font_rasterizer` が担当する描画パイプラインの全体像
- 有向二次曲線、解析積分、色合成の責務分担
- AA 戦略や fill rule と結び付く論点
- char から glyph、direction、width へ落ちる最小データモデル

## 現時点の要点

- `font_rasterizer` は GPU ベースのフォント描画を担当する中核クレートである
- AA は Windfoil の解析的な巻き数積分を採用している。通常経路は `fwidth` から局所ピクセル範囲を求める
- グリフには char 対応、glyph id、direction、width の軸があり、縦横レイアウトや幅解決と接続する
- `RasterizerRenderrer` は曲線 storage buffer を参照する矩形描画と、premultiplied color から straight-alpha への変換を行う
- `RasterizerPipeline` は renderer と screen/background/shader-art の最終表示段を束ねる
- `Quarity` は oversampling と GPU 上限考慮を持つ解像度ポリシーである
- `VectorVertexBuilder` は `OutlinePen` 実装として move/line/quad/cubic/close を有向二次曲線列へ変換する
- `StraightenOutlineBuilder` は glyph のアウトラインをサブパス単位で蓄積し、ほぼ直線の quad / line の連続を 1 本の line に簡約してから `VectorVertexBuilder` へ渡す
- `close` は開始点への区間だけを生成する。頂点タイプ・補助三角形・中心点探索・三角形インデックスは不要
- Windfoil は曲線を x/y の極値で単調分割し、水平行バンドと右端降順の並びでフラグメントの参照量を削減する
- 巻き数積分を non-zero / even-odd に応じて折り畳み、被覆率を求める。重複巻き数や透視変換などには制約が残る
- キャッシュも曲線列を版番号付き形式に保存し、旧頂点形式とは分離する
- `ttf_overlap_remover` は even-odd 前提の描画で元の non-zero 輪郭を再現するための前処理として存在する
- shader art は screen 前段の背景描画を差し替える別系統のパイプラインで、文字ラスタライズ本体とは独立している

## 現行実装での注意点

- `font_converter.rs` では overlap remover の有無にかかわらず `StraightenOutlineBuilder` を通す。Noto 系では overlap 除去後に直線簡約を行う
- debug shader は同じ `windfoil.wgsl` をディスクから読む。座標依存モーションは制御点を変形し、画面座標で再分割・積分する

## 過去の方式

Evan Wallace 型の扇形三角形、overlap count texture、outline resolve は旧方式である。関連する overlap / outline source と AA メモは過去の品質改善の記録として参照し、現行の描画経路とは区別する。