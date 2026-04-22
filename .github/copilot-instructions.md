# 炊紙（kashikishi）開発ガイド

炊紙は Rust と WebGPU(wgpu) で開発された三次元空間テキストエディタです。  
アーキテクチャの詳細は [`doc/project.md`](../doc/project.md) を参照してください。

## ドキュメント及びコミュニケーション

日本語を使用します。

## ビルド・テスト・Lint コマンド

| コマンド | 内容 |
|---|---|
| `mise r check` | fmt + clippy + clippy-tests をまとめて実行（CI と同等） |
| `cargo test --all` | 全テスト実行 |
| `cargo test -p <crate名> <テスト関数名>` | 単一テストを実行 |
| `mise r debug-kashikishi` | デバッグビルドで炊紙を起動（`RUST_LOG=info`） |
| `mise r kashikishi` | リリースビルドで炊紙を起動 |

コードを変更した際は必ず `mise r check` を通過させてください。  
開発は主に **Windows** 上で行われるため、Bash ではなく **PowerShell** を使用してください。

## コーディング規約

- エラー型は `thiserror` で定義する
- wasm32 向けの分岐は `#[cfg(target_arch = "wasm32")]` で記述する
- 新しい操作を追加する際は `ActionProcessor` トレイトを実装し `ActionProcessorStore::add_processor` で登録する
- 新しい画面（シーン）を追加する際は `World` トレイトを実装し `ModalWorld` で切り替える
