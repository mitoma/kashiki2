# Plan: ui_support examples ディレクトリのコンパイルエラー修正

すべての examples ファイルで `StateContext` が `UiContext` に変更されたことに伴うメソッドシグニチャの不一致エラーが発生しています。`aa_svg_test.rs` を参考に、各ファイルを修正します。

## 影響を受けるファイル

- `aa_test.rs` - 4箇所のメソッドシグニチャエラー
- `aa_test_rotate.rs` - 4箇所のメソッドシグニチャエラー
- `save_apng.rs` - 4箇所のメソッドシグニチャエラー
- `save_apng2.rs` - 4箇所のメソッドシグニチャエラー + 2箇所の型不一致エラー
- `save_gif.rs` - 4箇所のメソッドシグニチャエラー + 2箇所の型不一致エラー
- `setup_glyphs.rs` - 4箇所のメソッドシグニチャエラー
- `support_test.rs` - 4箇所のメソッドシグニチャエラー

## 修正手順

### Step 1: インポート文の修正

各ファイルで以下の変更を実施:
- `font_rasterizer::context::StateContext` のインポートを削除
- `ui_support::ui_context::UiContext` をインポートに追加

**参考パターン (aa_svg_test.rs より):**
```rust
use ui_support::{
    // ... other imports
    ui_context::UiContext,
};
```

### Step 2: メソッドシグニチャの修正

各ファイルの `SimpleStateCallback` トレイト実装で以下のメソッドを修正:

1. `init(&mut self, context: &StateContext)` → `init(&mut self, context: &UiContext)`
2. `update(&mut self, context: &StateContext)` → `update(&mut self, context: &UiContext)`
3. `input(&mut self, context: &StateContext, ...)` → `input(&mut self, context: &UiContext, ...)`
4. `action(&mut self, context: &StateContext, ...)` → `action(&mut self, context: &UiContext, ...)`

### Step 3: UiContext メソッド呼び出しの修正

`UiContext` はフィールドではなくメソッドでアクセスする必要があります:

- `context.device` → `context.device()`
- `context.queue` → `context.queue()`
- `context.color_theme` → `context.color_theme()`
- その他、context のフィールドアクセスがあればメソッド呼び出しに変更

**参考パターン (aa_svg_test.rs より):**
```rust
fn update(&mut self, context: &UiContext) {
    self.vectors
        .iter_mut()
        .for_each(|i| i.update_buffer(context.device(), context.queue()));
}
```

### Step 4: ファイル別の特別な考慮事項

#### save_apng2.rs と save_gif.rs
これらのファイルには `world.update(context)` と `action_processor_store.process(&action, context, &mut world)` の呼び出しがあります。メソッドシグニチャを修正すれば、これらの呼び出しも自動的に型が一致するはずです。

## 検証

すべての修正完了後:
```powershell
mise r check
```
を実行して、ui_support の examples がすべてコンパイルできることを確認します。
