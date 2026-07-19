---
title: Overlap Removal Strategy
kind: decision
status: draft
updated: 2026-07-19
source_refs:
  - ../../ttf_overlap_remover/src/lib.rs
  - ../../../memories/repo/font_overlap_artifact_notes.md
  - ../../../doc/ai-agent/plans/plan-changeRasterizerAlgorithm%20.md
related_pages:
  - ../components/overlap-remover.md
  - ./anti-aliasing-strategy.md
  - ../sources/source-overlap-remover-code.md
---

# Overlap Removal Strategy

## 現行位置づけ

- overlap remover は even-odd 系の描画互換を支える前処理として意味を持つ
- 同時に、non-zero + `front_facing` の shader 経路が進んでおり、長期的には overlap remover を不要化できる可能性がある

## この判断の理由

- 既存テストと絵文字品質検証は overlap remover 前提で高い一致率を出している
- 一方で shader 側には non-zero へ寄せる設計と実装の蓄積があり、責務重複がある
- 現時点で完全除去すると、品質検証・移行検証・既存 SVG / font 互換の確認が不足する

## 運用上の方針

- 当面は component として残し、historical / migration 文脈も保持する
- non-zero 経路の安定化と production 反映が済むまでは、削除対象ではなく比較基準として扱う

## 保留中の論点

- `font_converter.rs` から依存を外せる段階の判断
- overlap remover をライブラリとして残すか、検証資産に縮退させるか