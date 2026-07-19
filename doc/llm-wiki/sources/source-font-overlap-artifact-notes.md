---
title: Source Summary - Font Overlap Artifact Notes
kind: source
status: historical
updated: 2026-07-19
source_refs:
  - ../../../memories/repo/font_overlap_artifact_notes.md
related_pages:
  - ../decisions/anti-aliasing-strategy.md
  - ../concepts/text-rendering-pipeline.md
---

# Source Summary - Font Overlap Artifact Notes

## 対象 source

- `/memories/repo/font_overlap_artifact_notes.md`

## 要約

- `ttf_overlap_remover` v1 / v2 の知見と、Bezier 接続部 AA 漏れのデバッグ記録が併記されている
- debug shader では弦クリップで接続部の外側漏れと内部希薄化を改善し、さらに MAX coverage ブレンドで退行なく改善できた
- ただし production への反映は残っており、blend state と resolve の整合確認が必要である

## wiki への影響

- AA strategy の保留論点に「debug で確認済みだが production 未反映」の状態を記録できる
- overlap remover を独立 component / decision として後続で整理する入口になる