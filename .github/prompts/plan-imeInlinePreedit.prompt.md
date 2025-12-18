## Plan: IMEプレエディットのインライン表示

IMEの未確定文字列（プレエディット）をTextEdit内部でオンザフライ表示するために、入力イベントのルーティングをTextEditへ集約し、preedit状態を持たせてレイアウト・描画に統合します。描画は既存の文字幅計測とグリフ描画パイプラインを再利用し、色や下線などの強調を適用。最小構成でインライン表示を成立させ、追って候補ウィンドウ追従や下線デコレーションを拡張します。

### Steps
1. 入力ルーティング整備: winitのIMEイベントをUiEventへ拡張し、フォーカス中のTextEditへ伝搬（kashikishi/src/main.rs → ui_support/src/ui/textedit.rsへのImePreedit/ImeCommit/ImeCancel）。
2. TextEditにpreedit状態追加: Option<PreeditState>（文字列・選択範囲）とon_ime_preedit()/on_ime_commit()/on_ime_cancel()を実装（ui_support/src/ui/textedit.rs）。
3. プレエディットのレイアウト: 既存の幅計測と折返しを再利用しpreedit_char_statesを生成、キャレット位置から連結配置（font_rasterizer/src/char_width_calcurator.rsを利用、TextEdit内部の既存レイアウト手順に統合）。
4. インライン描画統合: 通常テキストの描画にpreedit_char_statesを重ねる。見た目は色変更（例: アクセント色）を適用、下線は後述の拡張で対応（font_rasterizer/src/glyph_instances.rs, font_rasterizer/src/vector_instances.rs, font_rasterizer/src/svg.rs）。
5. スクロール/カーソル維持: プレエディットは未確定のため本体バッファは変更せず、現行キャレット基準のスクロール維持ロジックを踏襲（ui_support/src/ui/textedit.rsのスクロール更新系に組込み）。
6. 候補窓追従（任意・推奨）: キャレットのウィンドウ座標を算出しset_ime_cursor_area相当を呼ぶ橋渡しAPIを追加（kashikishi/src/main.rs側でTextEditの座標→ウィンドウ座標変換を受け、winitへ反映）。

### Further Considerations
1. 下線表現: 下線表現は対応せず、色とブラケットで "今日は[いい天気]です" のようにプレエディット部分を表現します。
2. 互換運用: 既存の下部オーバーレイ表示は当面オプションで残し、設定で切替。
3. 複数エディタ: IMEフォーカスはfocusで一意化。フォーカス移動時にpreeditをクリア。
