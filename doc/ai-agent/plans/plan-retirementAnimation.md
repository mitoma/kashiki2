# 削除アニメーション（退場処理）リアーキテクチャ計画

## 概要

文字・キャレット・モデルなどの要素を削除するとき、炊紙では削除アニメーションを再生してから実際に破棄する。
しかし現状は要素の種類ごとに別々の仕組みで実装されており、対応漏れや不具合がある。
本計画では「退場中（retiring）の要素」を共通の仕組みで管理し、全ての削除経路で一貫した退場アニメーションを実現する。

## 現状

| 対象 | 現在の仕組み | 削除アニメーション |
|---|---|---|
| 文字 | `CharStates::char_to_dustbox` で `removed_chars` と `InstanceKey::PreRemovePosition` へ退避 | あり |
| キャレット | `removed_carets` に退避し、色を背景色へ寄せる | あり（文字とは別実装） |
| 通常モデル | `DefaultWorld::remove_current` で y を -5 し `removed_models` へ | 下へスライドのみ（フェードなし） |
| モーダル | `ModalModel` の `closing` と `opacity` | フェードあり（通常モデルとは別実装） |
| preedit 文字 | `sync_preedit_chars` で即削除 | なし |
| `SingleLine` | `instances.clear()` で作り直し | なし |
| 罫線（`ModelBorder::None`） | `border_states = None` | なし |
| メモのカテゴリ切替など | `reset_world` で `DefaultWorld::new` に置き換え | なし |
| モード切替（main.rs） | `self.world = world` | なし |

## 設計上の問題

1. **破棄のタイミングが削除アニメーションの長さに基づいていない**
   - ゴミ箱から消す条件は CPU 側 `motion_gain` イージングの終了だが、GPU 側の削除モーションは `remove_char.duration` と開始遅延（`counter * 10`）で決まる。
   - 一括削除時、遅延が CPU 側イージングの長さ（デフォルト 500ms）を超える文字（51 文字目以降）はモーション開始前に消える。
   - `motion_gain.update([0.0])` で無理やりアニメーション中にしているのもこのため（コード中の TODO 参照）。
2. **生存期間を `in_animation()` から推測している**
   - `TextEdit::in_animation` は位置・サイズ・回転しか見ないため、内部の文字アニメーションが続いていてもモデルが消える。
3. **退避キーが衝突する**
   - `PreRemovePosition(row, col)` と文字の組がキーなので、同じ位置の同じ文字を短時間に続けて消すと前の削除アニメーションが上書きされる。
4. **仕組みが 4 か所に重複している**（文字・キャレット・通常モデル・モーダル）。新しい対象を追加するときに対応が漏れやすい。
5. **`RemoveCharMode::Immediate` が特別扱い**になっている。本来は「退場時間 0」で表現できる。

## 設計方針

### 1. 退場中の要素を共通コンテナ `Retiring<T>` で管理する

```rust
pub(crate) struct RetireId(usize);

pub(crate) struct Retiring<T> {
    entries: Vec<RetiringEntry<T>>,
    next_id: usize,
}

impl<T> Retiring<T> {
    /// 要素を退場させ、一意な ID を返す
    fn retire(&mut self, value: T, deadline: u32) -> RetireId;
    /// deadline を過ぎた要素を取り除き、破棄処理を呼ぶ
    fn sweep(&mut self, now: u32, on_drop: impl FnMut(RetireId, T));
    fn iter_mut(&mut self) -> impl Iterator<Item = (RetireId, &mut T)>;
}
```

- 破棄時刻（deadline）は `max(GPU モーションの終了時刻, CPU イージングの終了時刻)` で明示的に計算する。`in_animation()` からの推測はしない。
- `font_rasterizer::time` の固定クロックで時刻を制御できるので、E2E とユニットテストで決定的に検証できる。
- GPU インスタンスのキーは `InstanceKey::Retired(usize)` の一意 ID にして、キーの衝突を解消する。
- `RemoveCharMode::Immediate` は deadline を `now` にするだけで表現する。

### 2. モデルの退場を World で一元管理する（`DepartingModel`）

```rust
struct DepartingModel {
    model: Box<dyn Model>,
    transition: ExitTransition, // opacity と offset のイージング、deadline
    layer: Layer,               // Normal | Modal
}
```

- 退場演出はまず `ModelRenderBatch.opacity` を使ったフェードとスライドとして World 側で共通化する。これで `Model` の実装に依存せず、全モデルが同じように退場する。
- モデル固有の演出が必要な場合だけ、`Model` トレイトに省略可能なフック `fn begin_exit(&mut self, ctx: &UiContext) -> Duration { Duration::ZERO }` を追加する。World は共通演出と固有演出の長い方を deadline にする。

### 3. 「作り直し」を「差し替え」に変える

- `reset_world` が毎回 `DefaultWorld::new` を作るのをやめ、`DefaultWorld::replace_models(new_models)` / `retire_all()` で旧モデルを `DepartingModel` に移す。
- モード切替では `KashikishiCallback` が `outgoing_world: Option<Box<dyn ModalWorld>>` を持ち、旧 World の `render_batches` に opacity を掛けて重ね描きし、退場が終わったら破棄する。
- `SingleLine` は前回との差分を取る形にして、`Retiring` に乗せる。

### 4. 演出（何をするか）と仕組み（いつ消すか）を分ける

- `CharEasings` は演出（motion / duration / gain）だけを定義する。
- いつ消すかは `Retiring` が deadline で決める。プリセットを追加しても、破棄タイミングの正しさに影響しない。

## 実装ステップ

各ステップで `mise r check` と `cargo test --all` が通る状態を保つ。

### Step 1: `Retiring<T>` を導入し、文字とキャレットを移行する

- `ui_support/src/retiring.rs` に `Retiring<T>` / `RetireId` と deadline 計算関数を追加する。
- `font_rasterizer::vector_instances::InstanceKey::PreRemovePosition` を `InstanceKey::Retired(usize)` に置き換える。
- `TextInstances` / `CaretInstances` の `pre_remove` / `get_mut_from_dustbox` / `remove_from_dustbox` を、`RetireId` を受け取る `retire` / `get_mut_retired` / `remove_retired` に置き換える。
- `CharStates::removed_chars` と `CaretStates::removed_carets` を `Retiring` にし、deadline で破棄する。
  - 文字の deadline: `max(instance.start_time + remove_char.duration, now + CPU イージングの最大 duration)`
  - キャレットの deadline: `max(now + remove_caret.duration, now + CPU イージングの最大 duration)`
- `motion_gain.update([0.0])` による強制アニメーション化と、`RemoveCharMode::Immediate` の特別な分岐を削除する（Immediate は deadline = now）。
- テスト: `Retiring` の deadline による破棄と ID の一意性、遅延開始のモーションで deadline が延びること。

### Step 2: `DepartingModel` でモーダルと通常モデルの退場を統合する

- `removed_models` と `ModalModel.closing` を `DepartingModel` に統合する。
- 通常モデルもフェードとスライドで退場させる。
- 既存テスト（`closing_modals_keep_their_layer_when_another_modal_opens` など）を新構造に合わせる。

### Step 3: `DefaultWorld::replace_models` / `retire_all` を追加し、`reset_world` を置き換える

- `CategorizedMemosWorld` / `TagbaseMemosWorld` の `reset_world` で旧モデルを退場させる。

### Step 4: モード切替時の World の入れ替え演出

- `KashikishiCallback` に `outgoing_world` を追加し、退場が終わるまで重ね描きする。

### Step 5: preedit・`SingleLine`・罫線を `Retiring` に移行する

## 検証

- 各ステップで `mise r check` と `cargo test --all`。
- 描画が変わるステップ（Step 2 以降）では `mise r e2e` を実行し、意図した差分であれば `mise r e2e-update` でスナップショットを更新する。
