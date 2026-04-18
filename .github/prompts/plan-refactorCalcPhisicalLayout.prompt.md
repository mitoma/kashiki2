# calc_phisical_layout メソッドのリファクタリングプラン

## 目的

[calc_phisical_layout](text_buffer/src/editor.rs#L190-L439) メソッド（約250行）を、重複コードを関数に切り出し、状態管理構造体を導入することで簡潔化する（目標: 120-150行程度）。

## 現状の問題点

1. **preedit文字列挿入処理の重複**: 3箇所で約50行のコードが重複（合計150行の重複）
   - 空行でのpreedit挿入
   - 通常行のキャレット位置でのpreedit挿入  
   - 行末でのpreedit挿入

2. **禁則文字判定ロジックの散在**: 4箇所に同じ条件分岐が存在
   - 各preedit挿入処理内（3箇所）
   - 通常のバッファ文字処理（1箇所）

## リファクタリング手順

### ステップ1: apply_line_break_rules メソッドの追加

**目的**: 禁則文字による改行判定ロジックを1箇所に集約

**実装内容**:
- `impl Editor` ブロック内にプライベートメソッドとして追加
- シグネチャ:
  ```rust
  #[inline]
  #[allow(clippy::too_many_arguments)]
  fn apply_line_break_rules(
      c: char,
      char_width: usize,
      is_line_head: bool,
      phisical_row: &mut usize,
      phisical_col: &mut usize,
      indent: usize,
      max_line_width: usize,
      line_boundary_prohibited_chars: &LineBoundaryProhibitedChars,
  )
  ```
- `phisical_row` と `phisical_col` の可変参照を受け取り直接更新（戻り値なし）
- 4箇所の重複コードをこのメソッド呼び出しに置き換え

**確認**: `mise r check` でテストが通ることを確認

### ステップ2: insert_preedit_chars メソッドの追加

**目的**: preedit文字列挿入処理を1箇所に集約

**実装内容**:
- プライベートメソッドとして追加
- シグネチャ:
  ```rust
  fn insert_preedit_chars(
      &self,
      preedit: &str,
      line_row_num: usize,
      caret_col: usize,
      phisical_row: &mut usize,
      phisical_col: &mut usize,
      indent: usize,
      max_line_width: usize,
      line_boundary_prohibited_chars: &LineBoundaryProhibitedChars,
      width_resolver: &Arc<dyn CharWidthResolver>,
      preedit_chars: &mut Vec<(BufferChar, PhisicalPosition)>,
  )
  ```
- 内部で `apply_line_break_rules` を呼び出す
- 3箇所の重複コード（約50行×3）をこのメソッド呼び出しに置き換え

**確認**: `mise r check` でテストが通ることを確認

### ステップ3: LayoutState 構造体の追加

**目的**: レイアウト計算中の状態を構造体にまとめる

**実装内容**:
- 構造体定義:
  ```rust
  struct LayoutState {
      chars: Vec<(BufferChar, PhisicalPosition)>,
      preedit_chars: Vec<(BufferChar, PhisicalPosition)>,
      phisical_row: usize,
      phisical_col: usize,
      main_caret_pos: PhisicalPosition,
      mark_pos: Option<PhisicalPosition>,
      preedit_injected: bool,
      main_caret_fixed: bool,
  }
  ```

- `impl LayoutState` に以下のメソッドを実装:
  - `new(mark: Option<Caret>) -> Self`: 初期化
  - `into_layout(self) -> PhisicalLayout`: 最終的な `PhisicalLayout` への変換

### ステップ4: calc_phisical_layout の書き換え

**目的**: `LayoutState` と抽出したメソッドを使用して処理フローを簡潔化

**実装内容**:
- `LayoutState::new()` で状態を初期化
- `insert_preedit_chars` のシグネチャを `LayoutState` を受け取る形に調整
- メソッド全体を書き換えて構造を整理

**確認**: `mise r check` でテストが通ることを確認

### ステップ5: 最終確認

**確認項目**:
- 全テストケースが通過
  - [test_calc_phisical_layout](text_buffer/src/editor.rs#L582)
  - [test_line_boundary_prohibited_chars](text_buffer/src/editor.rs#L652)
  - [test_indent](text_buffer/src/editor.rs#L720)
  - [test_preedit](text_buffer/src/editor.rs#L754)
- `cargo fmt` が問題なく完了
- `cargo clippy` で警告がない

## 設計上の決定事項

1. **関数のスコープ**: `Editor` のプライベートメソッドとして実装（`self` へのアクセスが必要な場合に対応）

2. **`LayoutState` の設計**: `into_layout()` メソッドを実装して `PhisicalLayout` への変換をカプセル化

3. **`apply_line_break_rules` の戻り値**: 戻り値なしで可変参照により直接更新（シンプルさ優先）

4. **段階的な進行**: 各ステップでテストを実行し、問題がないことを確認しながら進める

## 期待される効果

- **コード行数の削減**: 約250行 → 約120-150行（推定）
- **重複の排除**: 150行の重複コードを関数に集約
- **可読性向上**: 各ブロックの意図が関数名で明確化
- **保守性向上**: 禁則処理やpreedit処理のロジック変更が1箇所で済む
- **テスト容易性**: 各関数を個別にユニットテスト可能（将来的に）
