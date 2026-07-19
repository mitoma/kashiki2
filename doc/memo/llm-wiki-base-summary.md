# LLM Wiki ベース文書の要約

## 原文

- URL: https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f
- 参照名: `karpathy/llm-wiki.md`

## メモ

このファイルは原文の転載ではなく、LLM Wiki パターンの要点だけをこのリポジトリ向けに短くまとめたものです。

## 要点

- LLM Wiki は、raw source を毎回 RAG 的に再発見するのではなく、LLM が永続的な wiki を更新し続ける運用パターンである
- 基本構造は `raw sources`, `wiki`, `schema` の 3 層で整理される
- 主要操作は `ingest`, `query`, `lint` の 3 種類で、知識はチャットではなく markdown の永続成果物に戻される
- `index.md` と `log.md` が中規模運用での探索と履歴追跡の基点になる
- 人の役割は source の選定、観点の提示、意味づけであり、LLM は整理・更新・相互参照の保守を担う

## このリポジトリでの位置づけ

- `doc/llm-wiki/` 配下は、この考え方を炊紙の GPU テキストレンダリングとエディタ実装の知識ベースに具体化したもの
- 運用本体は `doc/llm-wiki/README.md`, `schema.md`, `index.md`, `log.md` と各 page 群に置く
- 原文が必要になった場合は gist 側を参照する