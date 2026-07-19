---
title: Ingest Workflow
kind: workflow
status: production
updated: 2026-07-19
source_refs:
  - ../schema.md
related_pages:
  - ../index.md
  - ../log.md
---

# Ingest Workflow

## 標準手順

1. raw source を 1 つ選ぶ
2. `sources/` に source 要約ページを作るか更新する
3. 関連する `concepts/`、`components/`、`decisions/` を更新する
4. `index.md` にページ追加と backlog 更新を反映する
5. `log.md` に ingest エントリを追記する

## 更新が必要になる条件

- レンダリング結果が変わった
- 設計判断が変わった
- public API や責務分担が変わった
- デバッグ / 検証手順が変わった

## source 要約に含めるべき要素

- source の目的
- 重要な主張または判断
- 関連するコードや設計判断
- 未解決点

## 注意点

- 1 source を大きくまとめすぎない
- historical な内容は削除せず status で区別する
- source 要約だけを作って関連ページを更新し忘れない