# タグベースのメモ帳の設計

https://github.com/mitoma/kashiki2/issues/80

今のメモ帳は 1 つの json ファイルで管理されておりおおよそ以下の構造を持っていた。

```json
{
    "current_category": "default",
    "categorized": {
        "category_name": {
            "created_at": "2024-06-18T12:34:56Z",
            "title": "メモのタイトル",
            "text": "メモの内容"
        }
    }
}
```

しかし、このようなメモ管理には以下の問題があった。

- カテゴリごとにメモが固定されてしまい複数のカテゴリにまたがるメモを管理しづらい
- 検索性が低く、目的のメモを見つけにくい
- 作成日時、編集日時といういった時系列に沿った管理がしづらい
- 一つの json ファイルに全てのメモが詰め込まれてしまい部分的なファイルの活用がしづらい

これらの問題を解決するために、タグベースのメモ管理を導入する。

## 要求

- メモの作成当初は日時の情報のみがあればよい
- タイトルやタグは後からつけたい
- 複数のタグを加えたい
- 無題メモを多数書きたい(scratch 的なメモにはタイトルを付けることも考えたくない)
- タグに関係なく時系列で眺められるようにしたい（emacs change-log の使い勝手が必要）
- 文字列を入力することで横断検索でメモを見つけられるようにしたい

## 設計案

- あるディレクトリをメモの場所として設定し、そのディレクトリ内のファイルレイアウトに従って管理する。

```
kashikishi-tagbase-memo.toml
memos/
  2026-06/
    2026-06-18-001.md
    2026-06-18-002.md
    2026-06-18-003.md
    2026-06-19-001.md
  2026-07/
    2026-07-01-001.md
    2026-07-02-001.md
```

kashikishi-tagbase-memo.toml の中身には現在の編集状況であったりとか設定情報を持たせる。

```toml
default_tag = "default"
current_narrowing_tag = ["todo", "scratch"]
```

このディレクトリ自体の履歴管理を Git などのバージョン管理システムで行うことでメモの変更履歴を追跡できるようにする。

これを tagbase memo としてライブラリ化し、メモの作成、編集、検索、タグ付けなどの操作を提供する。

{日付}-{連番}.md の形式でメモファイルを管理する。このメモには作成日時、更新日時、タイトル、タグ、本文などの情報を持つ必要がある。
例えば以下のように front matter を用いた形式で記述する。

```markdown
---
created_at: 2024-06-18T12:34:56Z
updated_at: 2024-06-18T12:34:56Z
title: メモのタイトル
tags: ["tag1", "tag2"]
---

メモの本文
```

### インターフェイスのイメージ

```rust
let memo_repository = TagbaseMemo::new("memos/");

// メモの作成
let memo = memo_repository.create_memo();

// メモの編集
memo_repository.update_memo(&memo, |m| {
    m.title = Some("新しいタイトル".to_string());
    m.tags.push("tag3".to_string());
});

// メモの検索
let search_results = memo_repository.search("キーワード");

// タグで絞り込み
let filtered_results = memo_repository.filter_by_tag("tag1");
```
