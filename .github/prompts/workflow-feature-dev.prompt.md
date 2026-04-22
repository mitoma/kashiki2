# 機能開発ワークフロー

新しい機能の開発を開始する際は、以下のステップに従ってください。

## Step 1: ブランチと worktree の作成

作業対象の機能名（英小文字・ハイフン区切り）を `<feature-name>` として使用します。

```powershell
# main の最新状態を取得
git fetch origin
git checkout main
git pull origin main

# ブランチと worktree を同時に作成（kashiki2 リポジトリと同じ階層に配置）
git worktree add ..\kashiki2-<feature-name> -b feature/<feature-name>
```

以降の作業は `..\kashiki2-<feature-name>` ディレクトリで行います。

## Step 2: 実装

worktree 内で実装を進めます。コミットは適切な粒度で積み上げてください。  
各コミットメッセージはコミット内容を日本語で簡潔に説明します。

```powershell
cd ..\kashiki2-<feature-name>
# ... 実装 ...
git add .
git commit -m "feat: <変更内容の要約>"
```

## Step 3: チェック

変更が完了したらチェックを実行します。

```powershell
mise r check   # fmt + clippy + clippy-tests
cargo test --all
```

すべてパスしていることを確認してください。

## Step 4: 開発者への確認

実装内容を開発者に提示し、承認を得ます。

- 変更の概要（何を・なぜ変えたか）を説明する
- `git --no-pager diff main` で差分を確認できるよう提示する
- 承認が得られるまで実装を続けます

## Step 5: Pull Request の作成

開発者の承認が得られたら PR を作成します。

```powershell
gh pr create --base main --title "<PRタイトル>" --body "<変更概要>"
```

PR タイトルは日本語で簡潔に、本文は変更内容・動作確認方法を記載します。

## Step 6: worktree の後片付け（マージ後）

PR がマージされたら worktree を削除します。

```powershell
cd ..\kashiki2
git worktree remove ..\kashiki2-<feature-name>
git branch -d feature/<feature-name>
```
