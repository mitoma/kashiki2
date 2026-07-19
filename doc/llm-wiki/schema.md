# LLM Wiki Schema

## 役割

`doc/llm-wiki/` は generated wiki であり、raw source そのものではない。
LLM は raw source を読み、この wiki を更新する。人は構造確認、優先順位付け、誤り修正を行う。

## ページ種別

### concepts

- 複数 source を横断して整理した概念説明
- 実装単位より広い視点を持つ
- 例: text rendering pipeline, layout engine, editor buffer model

### components

- 特定の実装単位、モジュール、データ構造の説明
- 例: vector vertex builder, glyph cache, preedit rendering

### decisions

- 採用済み、保留中、撤回済みの判断を記録する
- historical な試行錯誤はここに残す

### workflows

- ingest、調査、検証、lint の進め方
- 変更時に人と LLM がどう動くかを定義する

### sources

- raw source 単位の要約ページ
- 関連 concepts / components / decisions を更新するための入口

## 必須 frontmatter

各ページは原則として以下を持つ。

```yaml
---
title: ページ名
kind: concept | component | decision | workflow | source
status: production | draft | historical
updated: 2026-07-19
source_refs:
  - ../doc/or/code/path
related_pages:
  - ../relative-page.md
---
```

## status の意味

- `production`: 現行実装・現行運用に対応している
- `draft`: 計画中、検証中、まだ確定していない
- `historical`: 失敗案、revert 済み、過去の経緯として残すもの

## 更新ルール

1. 新しい source を読んだら、まず `sources/` に要約を作るか更新する
2. 関連する `concepts/`、`components/`、`decisions/` を更新する
3. `index.md` のページ一覧と backlog を更新する
4. `log.md` に ingest / query / lint を追記する
5. `production` と `draft` を同じページ内で曖昧に混ぜない

## 記述ルール

- raw source の丸写しではなく、判断材料と因果関係を要約する
- コード上の責務、制約、検証方法を優先して記述する
- 不具合修正や失敗案は「なぜそうしたか」と「何が駄目だったか」を残す
- 関連ページの相互リンクを必ず張る

## 初期優先順位

1. text rendering pipeline
2. layout engine
3. editor buffer model
4. anti-aliasing strategy
5. unicode segmentation decision
6. preedit rendering source of truth

## lint 観点

- orphan page がないか
- `source_refs` が空でないか
- `draft` のまま長期間放置されていないか
- 現行コード変更に対して stale な記述がないか
- `historical` な内容が `production` と誤読されないか