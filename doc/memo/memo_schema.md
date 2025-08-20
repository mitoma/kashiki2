# カテゴライズされたメモのスキーマ設計

https://github.com/mitoma/kashiki2/issues/80

## 要求

- カテゴリは後からつけたい
- メモの作成当初は日付が入っていたら十分
- 複数のカテゴリに加えたい（≒カテゴリというよりはタグなんだろう）
- 無題テキストを書き散らかしたい
- カテゴリ関係なく時系列で眺めていきたい（emacs change-log の使い勝手が必要）
- チラ見したいドキュメントを別カテゴリからチラ見できるようにしたい（≒オンデマンドなタグ）
- 横断検索で移動できるといいね
- タブみたいな概念必要かな？

## スキーマ設計

[スキーマ定義](./memo_schema.json)

## 必要な操作

Workspace
- add_memo
- remove_memo(memo_id)
- modify_tag(old_tag, new_tag)
- add_workspace_tag(tag)
- reset_workspace_tag()

Memo
- add_tag(tag)
- remove_tag(tag)
