## Plan: text_buffer 設計改善（互換重視）

外部利用が多い Editor/EditorOperation/ChangeEvent の公開面は維持しつつ、内部の責務分離・安全性・テスト容易性を段階的に改善する。まず panic 要因とイベント送信依存を減らし、次に action/editor の責務境界を整理し、最後に性能と拡張性を補強する方針。

**TODO (Progress)**
- [x] 公開利用点を棚卸しし、互換維持対象を明文化
- [x] text_buffer の互換ゴールデンテストを追加（イベント列・Undo/Redo・選択解除）
- [x] panic 要因の主要経路を防御（word移動、highlight変換、行結合、mark参照、sender切断）
- [x] action の巨大 match を機能別ハンドラへ分割（移動/編集/検索/クリップボード）
- [x] editor の選択状態責務を内部型へ抽出
- [x] ChangeEvent notifier の crate 内抽象化を導入
- [x] calc_indent の設定オブジェクト化の下地を導入（デフォルト互換維持）
- [x] UI連携回帰テストを追加（ui_support 側 bulk_change_events + Editor 実連携）
- [ ] highlight_positions / selection の性能計測と必要最小限の最適化
- [x] workspace 全体チェック（mise r check）

**Steps**
1. フェーズ1: 互換境界の固定化（最優先）
2. text_buffer の公開利用点を棚卸しし、互換維持対象を明文化する。対象は Editor::new, Editor::operation, Editor::to_buffer_string, Editor::buffer_chars, Editor::calc_phisical_layout, EditorOperation, ChangeEvent, Caret/CaretType, BufferChar/CellPosition。*この結果が以後の全工程の前提*
3. 既存挙動のゴールデンテストを追加する（イベント列・Undo/Redo・レイアウト・選択解除挙動）。将来の内部リファクタで外部挙動が変わらないことを先に保証する。*depends on 1*
4. フェーズ2: 安全性改善（panic削減、外部影響なし）
5. unwrap 起因の落ちうる経路を順次除去する。特に buffer の word移動、highlight位置計算、行結合処理、editor の mark 参照を Option/境界チェックで防御する。イベント送信失敗は内部ヘルパで握りつぶしまたはログ化し、UI終了時の panic を防ぐ。*depends on 3*
6. 文字検索系のバイト/文字インデックス変換を単一ヘルパに集約し、失敗時は該当マッチをスキップする方針で統一する。*parallel with 5*
7. フェーズ3: 責務分離（API維持で内部構造を改善）
8. action の巨大 match を機能別ハンドラへ分割する（移動系/編集系/選択系/検索系/Undo系）。既存の BufferApplyer::apply_action はファサードとして残し、内部で分配する。*depends on 5*
9. editor の選択状態（main_caret + mark）の更新責務を小さな内部型へ抽出し、selection差分通知ロジックを独立テスト可能にする。公開メソッド mark/unmark/operation は据え置く。*depends on 8*
10. フェーズ4: 可観測性と拡張性（中優先）
11. ChangeEvent 通知を内部 trait 経由に置換できる下地を作る（まずは crate 内限定の抽象化）。Editor::new(Sender<ChangeEvent>) は維持しつつ、内部で notifier をラップしてテスト容易性を高める。*depends on 5*
12. calc_indent と箇条書きパターンを設定オブジェクト化する準備を行う。現時点ではデフォルト値で既存挙動を完全維持し、公開API変更は次段階に送る。*parallel with 11*
13. フェーズ5: 性能確認と必要最小限の最適化
14. highlight_positions と selection 生成のコストを計測し、効果が確認できた場合のみキャッシュ/一時領域再利用を導入する。最適化は計測結果ベースで限定的に行う。*depends on 6,9*

**Relevant files**
- c:/Users/mutet/workspace/kashiki2/text_buffer/src/editor.rs — 公開API維持の中心。selection差分通知、mark管理、レイアウト計算の安全化と分離
- c:/Users/mutet/workspace/kashiki2/text_buffer/src/action.rs — apply_action の責務分割、Undo逆操作の整合性維持
- c:/Users/mutet/workspace/kashiki2/text_buffer/src/buffer.rs — word移動/検索/削除の境界処理、byte-char 変換安全化
- c:/Users/mutet/workspace/kashiki2/text_buffer/src/caret.rs — move通知の送信失敗耐性
- c:/Users/mutet/workspace/kashiki2/ui_support/src/ui/textedit.rs — Editor 公開面への依存確認（回帰確認対象）
- c:/Users/mutet/workspace/kashiki2/ui_support/src/ui/mod.rs — ChangeEvent の受信・集約挙動の回帰確認対象
- c:/Users/mutet/workspace/kashiki2/kashikishi/src/main.rs — 実アプリ側 EditorOperation 利用の回帰確認対象

**Verification**
1. text_buffer の既存テストを実行し、失敗ゼロを確認
2. text_buffer に追加した互換ゴールデンテスト（イベント列、Undo/Redo、選択解除）を実行
3. UI連携確認として ui_support 側の ChangeEvent 消費ロジックに対するテストまたは手動確認を実施
4. workspace 全体でチェックを実行（PowerShell 前提で mise r check）し、fmt/clippy/test を通す
5. 大きめテキストで highlight/move_to_next/move_to_previous の挙動と性能を確認（改善前後比較）

**Decisions**
- 含む範囲: text_buffer 内部設計改善、panic耐性、責務分割、互換テスト整備
- 含まない範囲: Editor の公開シグネチャ変更、ChangeEvent 列挙子の破壊的変更、マルチキャレット新機能追加
- 命名 typo（phisical -> physical）は互換影響を避けるため今回は内部 alias 追加まで。公開名の全面変更は別タスク
- 性能最適化は「計測で有意差が出る箇所のみ」に限定

**Further Considerations**
1. 送信失敗時の方針: 無視して継続を推奨。代替として debug 時のみログ出力を有効化 → この方針でいく。
2. 互換保証レベル: 同期テスト中心で十分か、イベント順序まで厳密一致させるかを実装前に選択 → イベント順序まで厳密に保証する方針でいく。
3. 将来拡張: notifier trait を先に入れておくと、非同期UI/録画再生機能への展開が容易 → 先に内部 trait を入れる方針でいく。
