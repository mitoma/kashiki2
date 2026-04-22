# StateContext の Senders を ui_support/UiContext に移動する計画

## ゴール
font_rasterizer::StateContext から UI 層に属する送信チャネル群 (Senders) を分離し、ui_support::UiContext が保持する構造へ再配置することで、責務境界を明確化し描画/状態管理層と UI イベント伝播層の分離を高める。

## 現状整理 (要点)
- Senders: 文字列, SVG, Action, PostAction の4種類の mpsc::Sender を保持。
- StateContext: GPU Device/Queue やフォント・方向・ウィンドウサイズ等の描画関連状態 + Senders を含む。
- UiContext: 現状 StateContext をラップしアクセサ/委譲メソッドを提供している。
- 依存方向: ui_support → font_rasterizer。StateContext に UI 的責務が混在。

## 変更方針
1. Senders 構造体を font_rasterizer から除去し ui_support に新設。
2. UiContext が StateContext と Senders を個別に所有する形へ変更。
3. StateContext から register_* 系メソッドを削除（UI 送信責務排除）。
4. UiContext に register_string / register_svg / register_action / register_post_action / action_sender / post_action_sender を再実装 (Senders 直接利用)。
5. StateContext::new から Senders 引数を削除し利用側初期化コード調整。
6. UiContext::new シグネチャを (StateContext, Senders) 受け取りに変更。
7. 生成箇所でのチャネル生成は引き続き ui_support 内で行い Senders::new → UiContext::new に渡す。
8. 移行後のビルド/テスト (cargo check) を通し影響範囲最小化を確認。

## 詳細ステップ
1. font_rasterizer/src/context.rs:
   - Senders 定義と関連メソッド (register_*/action_sender など) 削除。
   - StateContext フィールドから senders 削除。
2. ui_support/src/context.rs:
   - 新規または既存ファイル内に Senders 構造体定義追加。
   - UiContext フィールド: state_context: StateContext, senders: Senders。
   - 委譲メソッドを senders 呼び出しに差し替え。
3. UiContext::new 修正。
4. UiContext 初期化部 (現在 StateContext::new 呼び出し後に UiContext::new) を調整し、StateContext::new の引数から Senders を削除し、代わりに UiContext::new に送る。
5. 既存コードで UiContext 経由の register_* 呼び出しはそのまま動作 (内部実装変更のみ) を確認。
6. cargo fmt & cargo clippy & cargo test / cargo check を実行しコンパイルエラー洗出し。必要最小限の修正反映。

## 影響範囲 (想定)
- 直接変更: font_rasterizer/src/context.rs, ui_support/src/context.rs。
- 初期化: UiContext/StateContext を生成する箇所 (ui_support 内)。
- 利用側: 呼び出しシグネチャに変更なし → 既存利用箇所の修正不要見込み。

## リスク / 注意点
- 循環依存生成を避ける: Senders を ui_support に移動しても font_rasterizer が ui_support を参照しない設計を維持。
- Action 型は stroke_parser 由来のため引き続き共通利用可能。
- ログ warn! の呼び出し位置移動によるロギング挙動変化は軽微。

## 完了条件
- StateContext から UI チャネル関連コード排除。
- UiContext が Senders を保持し正常にイベント送信可能。
- cargo check が成功し clippy 警告 (許容範囲外) が新規に増えていない。
