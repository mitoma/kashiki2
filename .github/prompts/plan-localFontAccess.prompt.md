# Plan: Local Font Access API でユーザーフォント対応

Local Font Access API を使用してローカルフォントをバイナリで `run_wasm()` に渡すことは **技術的に十分可能**です。プロジェクトは既にバイナリフォント処理に対応しており、主な作業は JavaScript 側のフォント取得実装になります。

## Steps

1. [sample_codes/apng_gen/src](sample_codes/apng_gen/src) の `run_wasm()` 関数シグネチャを拡張し、フォントバイナリパラメータを追加

2. [site/src/custom_js/apng_gen.js](site/src/custom_js/apng_gen.js) に Local Font Access API で端末フォント一覧取得・選択機能を実装

3. 選択したフォントをバイナリに変換して `run_wasm()` に渡す処理を実装

4. ブラウザが API に未対応の場合、埋め込みフォント使用へのフォールバック機構を追加

5. HTML UI に「ローカルフォント選択」ドロップダウンを追加 ([site/src/apng_gen.md](site/src/apng_gen.md))

## Further Considerations

### 1. ブラウザ互換性

Chrome/Edge のみ対応。Firefox/Safari 非対応時の UX をどうするか？

→ 対応しない場合はデータを送らずに既存の埋め込みフォントを使用する。

### 2. セキュリティ

- 本番環境は HTTPS 必須
  → 問題なし
- ユーザー許可プロンプト対応の検討
  → 許可が得られない場合のフォールバック

## Technical Background

### run_wasm 現在のシグネチャ
```rust
pub async fn run_wasm(
    target_string: &str,
    window_size: &str,
    color_theme: &str,
    easing_preset: &str,
    fps: &str,
    transparent_bg: bool,
) -> Vec<u8>
```

### プロジェクト内のフォント処理状況
- Rust 側は既に `FontRepository::add_fallback_font_from_binary()` でバイナリフォント処理に対応
- WASM-bindgen で Uint8Array ↔ Rust データ型の変換が必要
- Local Font Access API は Promise ベースだが、既に async/await に対応

### ブラウザ対応
- Chrome/Edge: v92+ で対応（フラグで有効化可能）
- Firefox: 未対応
- Safari: 未対応

### 実装の難易度
**中程度** — Rust 側基盤は整備済み、JavaScript 側の実装が主な作業
