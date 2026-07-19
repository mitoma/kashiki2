# Kashikishi LLM Wiki

このディレクトリは、炊紙の GPU テキストレンダリング、レイアウト、エディタ実装に関する
知識を LLM が継続的に整理・更新するための wiki です。

## 目的

- raw source を毎回探索し直す代わりに、整理済みの知識を蓄積する
- 実装、設計判断、不具合調査の知見を継続更新できる形にする
- コード変更や新規メモを index / log / 関連ページへ反映する運用を定着させる

## レイヤー

1. Raw source
   - `doc/` 配下の文書
   - `font_rasterizer/`、`text_buffer/` などの Rust / WGSL 実装
   - `/memories/repo/` に保存された作業ノート
2. Wiki
   - この `doc/llm-wiki/` 配下の markdown
   - LLM が作成・更新し、人はレビューと方向付けを行う
3. Schema
   - [schema.md](schema.md)
   - ingest / query / lint の規約、frontmatter、更新ルールを定義する

## 最初に読むページ

- [schema.md](schema.md)
- [index.md](index.md)
- [log.md](log.md)

## 参考資料

- [../memo/llm-wiki-base-summary.md](../memo/llm-wiki-base-summary.md) LLM Wiki パターンの参考要約と原文 URL
- この `doc/llm-wiki/` 配下は、上記の一般的なアイデアを炊紙リポジトリ向けに具体化したもの

## ディレクトリ

- [concepts/](concepts/) 横断的な概念ページ
- [components/](components/) 実装単位の説明ページ
- [decisions/](decisions/) 採用・保留・撤回の設計判断
- [workflows/](workflows/) ingest / 検証 / 調査フロー
- [sources/](sources/) raw source の要約ページ

## 現在の方針

- 対象読者はこのリポジトリの開発者
- 初期フェーズからコードも source として扱う
- historical な失敗案も削除せず記録する
- source of truth は raw source に置き、wiki は整理済みの知識を担う