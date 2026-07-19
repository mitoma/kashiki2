# Index

このファイルは wiki の入口です。新しい query や更新では、まずここを見て関連ページを特定します。

## Core Pages

- [README.md](README.md) この wiki の目的とレイヤー
- [schema.md](schema.md) 更新規約とページ種別
- [log.md](log.md) ingest / query / lint の履歴

## Concepts

- [concepts/text-rendering-pipeline.md](concepts/text-rendering-pipeline.md) GPU テキスト描画の全体像
- [concepts/layout-engine.md](concepts/layout-engine.md) 折り返し、幅計算、preedit を含むレイアウト概念
- [concepts/editor-buffer-model.md](concepts/editor-buffer-model.md) Editor / Buffer / Caret / undo / ChangeEvent の構成
- [concepts/action-system.md](concepts/action-system.md) EditorOperation と reverse action の実行系
- [concepts/world-scene-architecture.md](concepts/world-scene-architecture.md) ModalWorld と World 切り替えの構成

## Components

- [components/glyph-model.md](components/glyph-model.md) char / glyph / direction / width の最小モデル
- [components/vector-vertex-builder.md](components/vector-vertex-builder.md) OutlineBuilder から GPU 頂点へ落とす変換器
- [components/overlap-remover.md](components/overlap-remover.md) even-odd 向けにパス重複を除去する幾何処理
- [components/shader-art-system.md](components/shader-art-system.md) 組み込み背景シェーダーと実行経路

## Decisions

- [decisions/anti-aliasing-strategy.md](decisions/anti-aliasing-strategy.md) 現行 AA と保留案
- [decisions/unicode-segmentation.md](decisions/unicode-segmentation.md) ICU4X 導入方針
- [decisions/preedit-source-of-truth.md](decisions/preedit-source-of-truth.md) preedit 配置の唯一の正
- [decisions/overlap-removal-strategy.md](decisions/overlap-removal-strategy.md) overlap remover と non-zero 移行の位置づけ

## Workflows

- [workflows/ingest-workflow.md](workflows/ingest-workflow.md) 新 source の取り込み手順
- [workflows/url-source-ingest.md](workflows/url-source-ingest.md) raw を置かない URL source の取り込み手順

## Sources

- [sources/source-project-overview.md](sources/source-project-overview.md) [../project.md](../project.md) の要約
- [sources/source-text-buffer-wrapping.md](sources/source-text-buffer-wrapping.md) [../text_buffer_wrapping.md](../text_buffer_wrapping.md) の要約
- [sources/source-anti-aliasing.md](sources/source-anti-aliasing.md) [../memo/anti_aliasing.md](../memo/anti_aliasing.md) の要約
- [sources/source-font-glyph-model.md](sources/source-font-glyph-model.md) [../memo/font.md](../memo/font.md) の要約
- [sources/source-text-buffer-design.md](sources/source-text-buffer-design.md) text_buffer 設計メモの要約
- [sources/source-vector-vertex-aa-analysis.md](sources/source-vector-vertex-aa-analysis.md) vector vertex AA 試行錯誤の要約
- [sources/source-font-overlap-artifact-notes.md](sources/source-font-overlap-artifact-notes.md) overlap / AA artifact メモの要約
- [sources/source-rasterizer-pipeline.md](sources/source-rasterizer-pipeline.md) rasterizer pipeline のコード要約
- [sources/source-text-buffer-layout-code.md](sources/source-text-buffer-layout-code.md) text_buffer layout 実装の要約
- [sources/source-text-buffer-editor-code.md](sources/source-text-buffer-editor-code.md) text_buffer editor 実装の要約
- [sources/source-text-buffer-action-code.md](sources/source-text-buffer-action-code.md) text_buffer action 実装の要約
- [sources/source-vector-vertex-builder.md](sources/source-vector-vertex-builder.md) vector vertex 生成実装の要約
- [sources/source-overlap-shader.md](sources/source-overlap-shader.md) overlap shader 実装の要約
- [sources/source-outline-shader.md](sources/source-outline-shader.md) outline shader 実装の要約
- [sources/source-overlap-remover-code.md](sources/source-overlap-remover-code.md) overlap remover 実装の要約
- [sources/source-shader-art-system.md](sources/source-shader-art-system.md) shader art 実装と利用経路の要約
- [sources/source-modal-worlds.md](sources/source-modal-worlds.md) kashikishi の World / scene 構成の要約
- [sources/source-evan-wallace-gpu-text-rendering.md](sources/source-evan-wallace-gpu-text-rendering.md) Evan Wallace の GPU text rendering 記事の要約
- [sources/source-frost-analytical-anti-aliasing.md](sources/source-frost-analytical-anti-aliasing.md) FrostKiwi の analytical anti-aliasing 記事の要約

## Backlog

### Tier 1

- 完了: `doc/memo/font.md` の source 化と glyph model component 追加
- 完了: `/memories/repo/text_buffer_design.md` の source 化と layout / preedit decision 反映
- 完了: `/memories/repo/vector_vertex_aa_analysis.md` の historical source 化
- 完了: `/memories/repo/font_overlap_artifact_notes.md` の source 化と AA decision 反映

### Tier 2

- 完了: `font_rasterizer/src/rasterizer_pipeline.rs` の source 化
- 完了: `font_rasterizer/src/vector_vertex.rs` の source 化
- 完了: `font_rasterizer/src/shader/overlap_shader.wgsl` の source 化
- 完了: `font_rasterizer/src/shader/outline_shader.wgsl` の source 化
- 完了: `text_buffer/src/editor.rs` の source 化
- 完了: `text_buffer/src/layout.rs` の source 化
- 完了: `text_buffer/src/action.rs` の source 化

### Tier 3

- 完了: shader art 系ページの追加
- 完了: overlap remover の component / decision 化
- 完了: World / scene 構成の concepts 化

## 運用メモ

- 新規ページを追加したら、この index に追記する
- backlog から消すときは、対応する source / concept / decision のどこに反映したかを log に残す