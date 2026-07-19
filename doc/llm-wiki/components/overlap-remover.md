---
title: Overlap Remover
kind: component
status: draft
updated: 2026-07-19
source_refs:
  - ../../ttf_overlap_remover/src/lib.rs
  - ../../ttf_overlap_remover/src/path_segment.rs
related_pages:
  - ../decisions/overlap-removal-strategy.md
  - ../sources/source-overlap-remover-code.md
  - ../concepts/text-rendering-pipeline.md
---

# Overlap Remover

## 概要

`ttf_overlap_remover` は、non-zero winding 前提のグリフ輪郭を even-odd でも同じ見た目になるよう変換するための幾何前処理である。

## アルゴリズムの骨格

- path を subpath 群へ分解する
- 全セグメント間の交差を検出し分割する
- winding number で左右の内外を判定して境界エッジだけを残す
- 残ったセグメントから loop と chain を再構成する
- Path に戻して出力する

## 実装上の特徴

- `PathSegment` が line / quadratic / cubic を抽象化する
- `split_all_segments` は AABB 早期棄却と交差点分割を行う
- `split_at_passing_endpoints` と `build_chains` は不完全な交差精度や未接続片への救済策である
- テストでは NotoEmoji の複数絵文字で Winding と EvenOdd の一致率を見ている

## 未整理の論点

- 本番描画から除去できる段階かどうか
- `front_facing` non-zero 経路との責務重複をどう解消するか