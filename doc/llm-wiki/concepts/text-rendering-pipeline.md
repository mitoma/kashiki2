---
title: Text Rendering Pipeline
kind: concept
status: draft
updated: 2026-11-14
source_refs:
  - ../../project.md
  - ../../memo/anti_aliasing.md
  - ../../memo/font.md
  - ../../font_rasterizer/src/rasterizer_pipeline.rs
  - ../../font_rasterizer/src/vector_vertex.rs
  - ../../font_rasterizer/src/straighten_outline_builder.rs
  - ../../font_rasterizer/src/straight_run_simplifier.rs
  - ../../../font_rasterizer/src/windfoil_renderrer.rs
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
- ベクトル形状、曲線の被覆率計算、resolve の責務分担
- AA 戦略や fill rule と結び付く論点
- char から glyph、direction、width へ落ちる最小データモデル

## 現時点の要点

- `font_rasterizer` は GPU ベースのフォント描画を担当する中核クレートである
- 描画は Windfoil に統一し、有向二次ベジェ曲線の解析的な積分から被覆率を求める
- グリフには char 対応、glyph id、direction、width の軸があり、縦横レイアウトや幅解決と接続する
- `RasterizerPipeline` は Windfoil の曲線投影・描画・resolve を実行する `RasterizerRenderrer` と、screen/background/shader-art の最終表示段を束ねる
- `Quarity` は oversampling と GPU 上限考慮を持つ解像度ポリシーである
- `VectorVertexBuilder` は move/line/quad/cubic/close を CPU 側の三角形列へ変換する。キャッシュ形式を維持し、その輪郭辺から Windfoil 用の曲線を復元する
- `StraightenOutlineBuilder` は glyph のアウトラインをサブパス単位で蓄積し、ほぼ直線の quad / line の連続を 1 本の line に簡約してから `VectorVertexBuilder` へ渡す
- `VectorVertexBuilder` は close 時にサブパスごとの中心点を origin として生成し、デフォルトでは最小角を最大化する中心点探索を使う
- GPU には曲線・水平バンド情報を登録し、Legacy 用の頂点/インデックスバッファは生成しない
- Legacy の overlap/outline shader と描画方式の選択 UI/API は廃止した。旧設定の `rendering_backend` は無視する
- `ttf_overlap_remover` は even-odd 前提の描画で元の non-zero 輪郭を再現するための前処理として存在する
- shader art は screen 前段の背景描画を差し替える別系統のパイプラインで、文字ラスタライズ本体とは独立している
- `World::render_batches` はモデル順を保持し、surface と content の ordered batch を RasterizerPipeline へ渡す
- Windfoil は batch 列を一つの premultiplied-alpha render pass で処理し、resolve を一度だけ行う。旧 per-batch resolve/composite fallback と専用テクスチャは削除した
- 面の前後関係は World 順で決め、深度バッファは使わない。normal と modal は独立した texture に描き、modal blur を保つ
- Windfoil ordered rendering は per-model full-screen composite を省き、debug phase 計測で command palette 約 20→1 ms/frame、tagbase 約 6→<1 ms/frame を確認した。計測は GPU timestamp ではない

## 現行実装での注意点

- `font_converter.rs` では overlap remover の有無にかかわらず `StraightenOutlineBuilder` を通す。Noto 系では overlap 除去後に直線簡約を行う
- debug shader モードでは `windfoil.wgsl` を実行時に読み込み、通常時は埋め込み済みの同じシェーダーを使う
- overlap/outline shader に関する source ページと過去の性能記録は Legacy 時代の資料であり、現行の描画経路ではない