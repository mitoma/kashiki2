## Plan: エディタ設定共有設計

設定ファイルの読み書き責務は kashikishi に残しつつ、TextContext 系の共有参照と実行時更新は ui_support 側の実行時設定へ寄せる。kashikishi_config.rs をそのまま共有型の中心にすると ui_support から参照できないため、永続化スキーマと実行時設定を分離するのが推奨。

**Steps**
1. 現状の設定項目を 2 層に分割する。アプリ起動時にしか使わない項目（font, ascii_override_font, background_shader など）と、UI/エディタから継続参照したい項目（TextContext の基本値、IME/SelectBox などの派生既定値、highlight/color theme 系）を整理する。
2. ui_support に editor 系の共有設定型を追加する。候補は EditorSettings / EditorProfiles / TextContextSeed のような構成で、TextContext を直接 serde させるのではなく「永続化しやすい値」と「TextContext への変換」を分ける。*この層は TextContext の base defaults と widget/profile 別 override を表現する。*
3. kashikishi/src/kashikishi_config.rs はファイル I/O と serde 用の AppConfig に寄せる。構成は app/font/background/ime と editor/appearance を分離し、ui_support 側の EditorSettings へ変換する入口を持たせる。*depends on 1,2*
4. ui_support の実行時共有参照点として UiContext に editor settings を追加する。StateContext は font_rasterizer の描画基盤型なので肥大化させず、UiContext が StateContext と editor settings を並列保持する。実装形は最初は所有値または Arc 共有で十分で、既存の accessor パターンに合わせて getter を増やす。*depends on 2*
5. TextContext の生成 API を見直す。TextEdit::default() 依存を減らし、TextEdit::new_with_config あるいは UiContext 経由の factory を導入する。各所の直接 Default 構築を、context.editor_settings() から base config を取得して必要な override だけ足す形へ移す。対象は ui_support/src/ui/textedit.rs, text_input.rs, ime_input.rs, card.rs, selectbox.rs。*depends on 4*
6. widget ごとの既定値は named profile 化する。例: document, modal_input, selectbox_search, selectbox_item, ime_preedit, card。永続設定ファイルには optional override として持たせ、未指定時は base defaults から派生する。これで TextContext の初期値と補助 UI の初期値が同じ設定体系に入る。*parallel with 5 after 2*
7. 実行時変更フローを定義する。既存の InputResult -> RenderState 更新パターンに合わせ、editor settings 更新用の結果型または action processor を追加する。更新時は UiContext 内の共有設定を差し替え、必要に応じて World 側へ再適用する。新規 TextEdit は常に最新設定を使い、既存 TextEdit は「全件再適用」対象と「生成時のみ適用」対象を分ける。*depends on 4,5*
8. color theme は persisted config では editor/appearance 配下にまとめてよいが、実行時の唯一の正は既存どおり color_theme を保持する側に残す。つまり「ファイル上は統合」「ランタイムでは責務ごとに保持」を明確にし、二重管理を避ける。*depends on 3,4*
9. 段階導入順を決める。第一段階は editor base defaults を UiContext 共有化して TextContext 初期値を外出し、第二段階で named profile を追加、第三段階で実行時更新と保存をつなぐ。これで差分が小さく検証しやすい。*depends on 3,4,5*

**Relevant files**
- c:/Users/mutet/workspace/kashiki2/kashikishi/src/kashikishi_config.rs — 設定ファイルの serde スキーマ、load/save、ui_support 型への変換入口
- c:/Users/mutet/workspace/kashiki2/kashikishi/src/main.rs — 起動時ロード、SimpleStateSupport / callback 初期化、設定保存タイミング
- c:/Users/mutet/workspace/kashiki2/ui_support/src/ui_context.rs — TextContext 定義、UiContext accessor、editor settings の共有参照点
- c:/Users/mutet/workspace/kashiki2/ui_support/src/render_state.rs — UiContext 初期化、InputResult ベースの実行時反映
- c:/Users/mutet/workspace/kashiki2/ui_support/src/lib.rs — SimpleStateSupport 拡張、handle_action_result の更新分岐
- c:/Users/mutet/workspace/kashiki2/ui_support/src/ui/textedit.rs — TextEdit::default から new_with_config / shared defaults への移行起点
- c:/Users/mutet/workspace/kashiki2/ui_support/src/ui/text_input.rs — base defaults + modal_input profile の適用
- c:/Users/mutet/workspace/kashiki2/ui_support/src/ui/ime_input.rs — ime_preedit profile の適用
- c:/Users/mutet/workspace/kashiki2/ui_support/src/ui/card.rs — card profile の適用
- c:/Users/mutet/workspace/kashiki2/ui_support/src/ui/selectbox.rs — search/item profile の適用
- c:/Users/mutet/workspace/kashiki2/font_rasterizer/src/context.rs — StateContext の責務境界確認。editor settings はここへ入れない前提
- c:/Users/mutet/workspace/kashiki2/doc/ai-agent/plans/plan-stateContextSendersMigration.md — UiContext に UI 責務を寄せる既存方針の参照

**Verification**
1. editor settings の共有化後、TextContext::default 直呼び箇所が意図した最小限だけ残っていることを検索で確認する。
2. 起動時に config.json から base defaults が読み込まれ、StartWorld や TextInput 生成時に同じ値が反映されることを手動確認する。
3. 実行中変更を追加した段階では、設定更新アクション後に新規生成 UI と既存対象 UI の反映差分が設計どおりかを確認する。
4. PowerShell で mise r check を実行して fmt/clippy/tests を通す。

**Decisions**
- kashikishi_config.rs はベース候補ではあるが、共有参照の中心型にはしない。理由は依存方向が kashikishi -> ui_support であり、ui_support から参照できないため。
- TextContext そのものを永続化スキーマにせず、永続化用 settings と実行時 TextContext を変換でつなぐ。理由は Direction / HighlightMode / preset / optional override の表現を安定させやすいため。
- StateContext ではなく UiContext に editor 系共有設定を置く。理由は StateContext が font_rasterizer 側の低レイヤー型で、既存方針でも UI 責務を外へ出しているため。
- 実行時更新は「共有設定の更新」と「既存モデルへの再適用」を分けて扱う。すべてを即時一括反映しようとすると影響範囲が広すぎるため、まずは生成時適用 + 明示的再適用対象から始める。
- スコープに含む: TextContext 基本値、IME/SelectBox 等の補助 UI 既定値、色テーマ/ハイライト既定値、フォント/背景設定との同一ファイル管理。
- スコープ外: 設定 UI 自体の完成、watch ベースの自動リロード、すべての既存 World への強制一括ライブ再構成。

**Further Considerations**
1. shared settings 型の配置先は ui_support 内で十分だが、将来 CLI や別アプリからも共有したいなら専用小クレート化を再検討する。
    - 現段階では ui_support 内で実装する。
2. 既存 TextEdit への再適用ポリシーは項目別に分ける。max_col や interval は再レイアウト可能だが、font/background は別経路で反映する方が安全。