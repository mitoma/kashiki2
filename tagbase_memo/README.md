# tagbase_memo

Markdown ファイルと YAML front matter で管理する、タグベースのメモ保管庫。
炊紙の GUI には依存せず、ローカルファイルシステム上で動作します。

## 利用例

```rust
use tagbase_memo::{Result, SortOrder, TagbaseMemo};

# fn main() -> Result<()> {
# let directory = tempfile::tempdir().unwrap();
# let root = directory.path();
let repository = TagbaseMemo::new(root)?;
let memo = repository.create_memo()?;
let updated = repository.update_memo(&memo.id, |content| {
    content.title = Some("開発メモ".to_owned());
    content.text = "今日の作業内容".to_owned();
    content.tags.push("todo".to_owned());
})?;
assert_eq!(repository.get_memo(&memo.id)?, updated);

let results = repository.search("作業 tag:todo")?;
assert_eq!(results.len(), 1);
let chronological = repository.list_memos_with_order(SortOrder::Asc)?;
assert_eq!(chronological.len(), 1);

repository.delete_memo(&memo.id)?;
# Ok(())
# }
```

`TagbaseMemo::new(root)` は保管庫ルートを受け取ります。必要なディレクトリと
`kashikishi-tagbase-settings.toml` がなければ作成し、既存設定が不正な場合は
上書きせずエラーを返します。設定は開くときに読み込み、`settings()` で参照できます。
設定ファイルを外部で変更した場合は保管庫を開き直します。

## データと API

- メモは `memos/YYYY-MM/YYYY-MM-DD-NNN.md` に保存します。連番は既存の最大値の次から
  最低3桁で採番し、1000以降も使用できます。既存ファイルは上書きしません。
- `Memo` は相対パスの `id`、`created_at`、`updated_at`、`content` を持ちます。
  `MemoContent` の `title`、`tags`、`text` が編集対象です。
  更新クロージャには内容だけを渡し、ID と日時はライブラリで管理します。
- `get_memo` は指定 ID のメモを直接取得します。非表示タグを持つメモも取得可能です。
- `create_memo` は空の無題メモを永続化し、既定タグ `scratch` を付けます。
  タグ候補は制約ではなく、既定タグも作成後に削除できます。
- `update_memo` はディスクから最新のメモを読み、内容が変更された場合のみ更新日時と
  ファイルを更新します。更新後の `Memo` を返し、本文の先頭・末尾空白は保持します。
- `delete_memo` はファイルを削除します。存在しない ID への操作はエラーです。
- `list_memos`、`search`、`filter_by_tags` は作成日時の降順で返します。
  それぞれの `_with_order` 版で `SortOrder::Asc` / `Desc` を指定できます。
  同じ日時のメモはファイル名の昇順、同じファイル名の場合は ID の昇順です。
- 保管庫内の相対パスが ID です。外部で名前を変えたファイルは別の ID として扱います。
  メモファイルは `memos/` 配下から再帰的に取得し、拡張子 `.md` 以外は無視します。
  親ディレクトリ参照、絶対パス、保管庫内のシンボリックリンクは拒否します。

front matter の必須項目は `created_at`、`updated_at`、`tags` です。
日時は UTC オフセット付き RFC 3339 形式、無題は `title` の省略、タグなしは `tags: []` で表します。
LF / CRLF を読み込み、保存時は front matter を LF で再生成します。
閉じ区切り直後の空行を1行だけ front matter と本文の区切りとして扱い、それ以降は本文として保持します。

## 検索クエリ

| 入力 | 動作 |
|---|---|
| `作業` | タイトル・本文の部分一致検索 |
| `作業 tag:todo tag:scratch` | キーワードと両タグの AND 検索 |
| `"my phrase"` | 空白を含む文字列を一つのキーワードとして検索 |
| `"tag:example"` | タイトル・本文の文字列 `tag:example` を検索 |
| `tag:"my tag"` | 空白を含むタグ名で絞り込み |
| `tag:"tag:example"` | タグ名 `tag:example` で絞り込み |
| `tag:archive` | 通常非表示の `archive` メモを明示して検索 |
| 空文字・空白のみ | `default_tag` で絞り込み |

キーワードは Unicode の小文字化による大文字・小文字を区別しない比較で、複数キーワードは
AND 条件です。タグは大文字・小文字を区別する完全一致です。
引用符内の `\"` と `\\` はそれぞれ引用符とバックスラッシュを表します。
閉じていない引用符、不正なエスケープ、空の引用文字列、値のない `tag:` は
`Error::InvalidQuery` を返します。

`filter_by_tags(&[])` も `default_tag` で絞り込みます。キーワードだけの検索には
既定タグの条件を追加しません。タグによる制約なしで眺める場合は `list_memos` を使用します。

`invisible_tag`（既定値 `archive`）を持つメモは一覧・検索・絞り込みから除外します。
そのタグをクエリまたはタグ絞り込み API で指定したときだけ表示対象となり、ほかの条件も適用します。

## 保存と制限

一時ファイルへの書き込み・同期後、ファイルを置き換えて保存します。
I/O エラー、設定や front matter の解析エラーはパスを含む `Error` で返します。
不正なメモを黙って読み飛ばすことはしません。

外部更新との競合検出、複数プロセスの同期、GUI 状態の保存、Git 操作、旧形式からの移行は
対象外です。外部更新を上書きする可能性があります。
ユーザー状態の保存と `.gitignore` の管理は利用側で行います。

詳細は [設計メモ](../doc/memo/tagbase_memo.md) を参照してください。
