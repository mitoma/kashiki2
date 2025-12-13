# CharEasings のプリセット追加計画

## 概要

`CharEasings` 構造体にポップ、クール、その他のスタイルを持つプリセットを追加します。既存の `default()`、`zero_motion()`、`ignore_camera()` メソッドと同じパターンで、新しいプリセットメソッドを実装します。

## 実装ステップ

### 1. CharEasings impl ブロックに5つの新しいプリセットメソッドを追加

ファイル: `ui_support/src/ui_context.rs` の `CharEasings` impl

#### プリセット一覧

##### `poppy()`: ポップで弾むような動き
- **特徴**: Elastic, Bounce を活用した楽しく弾むアニメーション
- **使用シーン**: カジュアルなUI、ゲーム、子供向けアプリ
- **設定**:
  - `add_char`: Elastic + MOVE_Y_PLUS + STRETCH_X_PLUS, duration: 600ms, gain: 1.2
  - `move_char`: Bounce + MOVE_Y_PLUS (TURN_BACK), duration: 400ms, gain: 0.8
  - `remove_char`: Elastic + MOVE_Y_MINUS + ROTATE_Z_PLUS, duration: 600ms, gain: 1.2
  - `select_char`: Elastic + ROTATE_Y_PLUS + STRETCH_X_PLUS, duration: 400ms, gain: 1.5
  - `notify_char`: Bounce + STRETCH_Y_PLUS + STRETCH_X_PLUS (TURN_BACK), duration: 600ms, gain: 4.0

##### `cool()`: クールで滑らかな動き
- **特徴**: Circ, Quad を活用した洗練された滑らかなアニメーション
- **使用シーン**: ビジネスアプリ、プロフェッショナルツール、ダッシュボード
- **設定**:
  - `add_char`: Circ (EaseOut) + MOVE_X_PLUS, duration: 400ms, gain: 0.6
  - `move_char`: Quad (EaseInOut) + MOVE_Y_PLUS, duration: 250ms, gain: 0.3
  - `remove_char`: Circ (EaseIn) + MOVE_X_MINUS + STRETCH_X_MINUS, duration: 400ms, gain: 0.6
  - `select_char`: Circ (EaseInOut) + ROTATE_Y_PLUS, duration: 300ms, gain: 0.8
  - `notify_char`: Quad (EaseInOut) + STRETCH_X_PLUS (TURN_BACK), duration: 400ms, gain: 2.0

##### `energetic()`: エネルギッシュで素早い動き
- **特徴**: Back, Expo を活用したダイナミックで力強いアニメーション
- **使用シーン**: アクション重視のUI、通知、アラート
- **設定**:
  - `add_char`: Back (EaseOut) + MOVE_Y_PLUS + ROTATE_Z_MINUS, duration: 350ms, gain: 1.0
  - `move_char`: Expo (EaseInOut) + MOVE_Y_PLUS (TURN_BACK), duration: 200ms, gain: 0.7
  - `remove_char`: Expo (EaseIn) + MOVE_Y_MINUS + ROTATE_Z_PLUS + STRETCH_X_MINUS, duration: 300ms, gain: 1.0
  - `select_char`: Back (EaseOut) + ROTATE_Y_PLUS + STRETCH_X_PLUS, duration: 250ms, gain: 1.2
  - `notify_char`: Expo (EaseOut) + STRETCH_Y_PLUS + STRETCH_X_PLUS + ROTATE_Z_PLUS (TURN_BACK), duration: 400ms, gain: 3.5

##### `gentle()`: 優しくゆったりした動き
- **特徴**: Sin, Cubic を活用した柔らかくリラックスした長めのアニメーション
- **使用シーン**: リーディングアプリ、瞑想アプリ、リラックス系コンテンツ
- **設定**:
  - `add_char`: Sin (EaseOut) + MOVE_Y_PLUS (TO_CURRENT), duration: 800ms, gain: 0.5
  - `move_char`: Cubic (EaseInOut) + MOVE_Y_PLUS, duration: 600ms, gain: 0.3
  - `remove_char`: Sin (EaseIn) + MOVE_Y_MINUS + STRETCH_X_MINUS, duration: 800ms, gain: 0.5
  - `select_char`: Cubic (EaseInOut) + ROTATE_Y_PLUS, duration: 500ms, gain: 0.7
  - `notify_char`: Sin (EaseInOut) + STRETCH_Y_PLUS + STRETCH_X_PLUS (TURN_BACK), duration: 700ms, gain: 2.0
  - CPU easing duration: 800ms

##### `minimal()`: ミニマルで控えめな動き
- **特徴**: Quad のみを使用した短時間で小さな動き
- **使用シーン**: データ重視のアプリ、コンソール、ターミナル風UI
- **設定**:
  - `add_char`: Quad (EaseOut) + MOVE_Y_PLUS, duration: 200ms, gain: 0.3
  - `move_char`: Quad (EaseInOut) + MOVE_Y_PLUS, duration: 150ms, gain: 0.2
  - `remove_char`: Quad (EaseIn) + MOVE_Y_MINUS, duration: 200ms, gain: 0.3
  - `select_char`: Quad (EaseInOut) + ROTATE_Y_PLUS, duration: 200ms, gain: 0.5
  - `notify_char`: Quad (EaseInOut) + STRETCH_X_PLUS (TURN_BACK), duration: 250ms, gain: 1.5
  - CPU easing duration: 200ms

### 2. プリセット取得用の enum とインターフェイスを追加

ファイル: `ui_support/src/ui_context.rs`

```rust
/// CharEasings のプリセットを指定するための enum
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharEasingsPreset {
    /// デフォルトのアニメーション設定
    Default,
    /// 動きのない設定
    ZeroMotion,
    /// カメラの影響を受けない設定
    IgnoreCamera,
    /// ポップで弾むような動き (Elastic, Bounce 中心)
    Poppy,
    /// クールで滑らかな動き (Circ, Quad 中心)
    Cool,
    /// エネルギッシュで素早い動き (Back, Expo 中心)
    Energetic,
    /// 優しくゆったりした動き (Sin, Cubic 中心、長めの duration)
    Gentle,
    /// ミニマルで控えめな動き (Quad のみ、短めの duration)
    Minimal,
}
```

`CharEasings` impl に追加:

```rust
/// プリセットから CharEasings を生成する
pub fn from_preset(preset: CharEasingsPreset) -> Self {
    match preset {
        CharEasingsPreset::Default => Self::default(),
        CharEasingsPreset::ZeroMotion => Self::zero_motion(),
        CharEasingsPreset::IgnoreCamera => Self::ignore_camera(),
        CharEasingsPreset::Poppy => Self::poppy(),
        CharEasingsPreset::Cool => Self::cool(),
        CharEasingsPreset::Energetic => Self::energetic(),
        CharEasingsPreset::Gentle => Self::gentle(),
        CharEasingsPreset::Minimal => Self::minimal(),
    }
}
```

### 3. 各プリセットのドキュメントコメントを追加

ファイル: `ui_support/src/ui_context.rs`

各プリセットメソッドに以下のようなドキュメントコメントを追加:

```rust
/// ポップで弾むような動きのプリセット。
///
/// Elastic と Bounce を活用した楽しく弾むアニメーション。
/// カジュアルなUI、ゲーム、子供向けアプリに適しています。
pub(crate) fn poppy() -> Self { ... }
```

## 検討事項

### 1. プリセットの命名
- 現在は英語名を提案 (`poppy`, `cool`, `energetic`, `gentle`, `minimal`)
- 日本語命名も検討可能 (例: `hajikeru()`, `cool()`, `genki()`, `yasashii()`, `minimal()`)
- → 一旦英語名で行きましょう

### 2. プリセットの数
- 提案: 5つのプリセット (Poppy, Cool, Energetic, Gentle, Minimal)
- 必要に応じて追加・削除可能
- → 5つでスタート

### 3. TextContext への統合
- `TextContext` 構造体にもプリセット選択機能を追加するか?
- 例: `with_char_easings_preset(preset: CharEasingsPreset)` メソッド
- → これは実装だけ追加しておく。

### 4. pub(crate) vs pub
- 現在のメソッドは `pub(crate)` だが、プリセットは外部公開する可能性
- enum `CharEasingsPreset` は `pub` にする
- プリセットメソッド群は `pub(crate)` のまま、`from_preset()` で公開
- → プリセットメソッド群は `pub(crate)` のまま、`from_preset()` で公開の方針で

### 5. カスタマイズ性
- プリセットをベースにカスタマイズする方法を提供するか?
- 例: `poppy().with_duration_multiplier(1.5)` のような builder パターン
- → 将来的な拡張として検討

## 実装の優先順位

1. **高**: 5つのプリセットメソッド実装
2. **高**: `CharEasingsPreset` enum と `from_preset()` メソッド
3. **中**: ドキュメントコメント追加
4. **低**: `TextContext` への統合
5. **低**: カスタマイズ機能の追加
