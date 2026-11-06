# Model Bound Surface 設計

## 目的

モデルの bound に沿った面を描き、手前にあるモデルの文字が背後のモデルの文字と混ざって読みにくくなる問題を抑える。面はモデルの位置・回転・bound と同じ変換に追従する。

## 採用する描画モデル

- 交差する面を正確に解決する深度バッファは導入しない。モデルは平面上の UI 要素として扱い、World が持つモデル順で描く。
- 描画単位をモデルごとの batch にする。1 batch は surface と既存の content（vector の border / caret、glyph）を分けて持つ。
- 通常モデルは World のモデル列順に合成する。後ろのモデルが前面になる。削除アニメーション中のモデルは現在の描画順を維持し、modal batch は通常 World の合成後に描く。
- 背景だけを全モデル分まとめて先に描くことはしない。そうすると、背面モデルの glyph が前面モデルの背景より後に描かれて透けてしまう。
- 面の矩形はモデルと同一の現在の変換・bound を利用する。bound や位置のアニメーション中も、文字・border とずれないこと。
- surface の camera detail は TextEdit の glyph easing 設定から引き継ぐ。Card など camera-ignore overlay の面だけが通常 camera で投影されないようにする。

## モデル API と既定値

- border と surface は独立したモデル属性にする。border なしの面や、角丸面と角丸 border を同じ境界定義から描く将来案を妨げない。
- 初期の surface style は `None` とテーマ背景色を表す値を持つ。色はテーマから取得し、カスタム色と opacity は後続拡張に備えて style の責務に置く。
- テキストモデルではテーマ背景面を既定で有効にし、面の遮蔽を利用者が別途設定しなくてもモデル重なりの可読性を改善する。SVG などの非テキストモデルは既定で面を持たない。
- 背景色は `ColorTheme::background()` に追従する。モデルごとの色指定は別途 API で可能にし、グローバルな画面背景設定とは混同しない。

## 描画経路の変更

現状の描画 API は World から vector と glyph を別々に集約し、Rasterizer は全 vector の後に全 glyph を描く。深度テストもない。このため vector に矩形を足すだけでは、モデル単位の遮蔽にならない。

World からモデル順の render batch を渡す。RenderState は各モデルの surface と content を順序付き `Buffers` に変換する。

- Windfoil は全 batch を一度の render 呼び出しに集める。全 shape を一つの premultiplied-alpha render pass でモデル順に描き、一度だけ straight-alpha resolve して通常 / modal の outline texture へ出す。per-model resolve と全画面 composite は行わない。
- Legacy は shape の coverage/count resolve がモデル間で混ざらないよう、従来どおり batch ごとに resolve し、全画面 compositor へ蓄積する。
- 通常 World と modal は別々に描き、最終段で modal 背景 blur と modal の合成を維持する。

モデル単位の描画データは借用参照のままとし、文字・vector の GPU buffer を複製しない。既存の単一描画コールバックは単一 batch として扱う。StackLayout 等の複合モデルは現行の描画順と内容を保ち、子単位の surface は子 batch を明示できる場合に限る。

## 初期スコープと拡張

初期実装は不透明なテーマ背景、矩形面、モデル順の遮蔽までとする。

- 角丸: surface と border が共有する形状パラメーターを使う。
- 半透明: batch 順の alpha 合成を維持し、instance の opacity と coverage を分離する。色のみの変更で実装しない。
- border と境界共有: surface / border の共通 bound・corner radius を使い、個別の寸法計算による隙間を避ける。
- 任意に交差する面の正確な可視性: 対象外。モデル順が前後関係の定義となる。

## 検証

- E2E シナリオで複数モデルが画面上で重なる状態を作り、PNG snapshot により前面モデルの背景が背面文字を覆い、前面モデル自身の文字は見えることを確認する。
- 既存 E2E の text snapshot が変わらないことも確認する。描画順・既定背景の変更による PNG 差分は意図をレビューして更新する。
- bound / position の遷移後の snapshot も確認し、背景・文字・border の境界が一致することを確かめる。
- 画像比較は `mise r e2e` を使い、golden 更新時だけ `UPDATE_SNAPSHOTS=1 mise r e2e` を実行する。

## 実装ノート

Windfoil ordered render の debug 計測では、command palette の `render 4: run all stage` が旧経路の約 20 ms/frame から約 1 ms/frame へ短縮した。tagbase memo でも約 6 ms/frame から 1 ms 未満になった。phase 計測は GPU timestamp ではなく command encoding を含む wall time なので、絶対値は環境依存の目安として扱う。E2E の一部 PNG は resolve 回数の減少に伴う AA / 量子化差が許容閾値をわずかに超えたため、実画像を確認して golden を更新した。

World API は vector/glyph を別々に公開していたため、順序付き batch を `ui_support` の `World` / `RenderData` から `font_rasterizer` の `RasterizerPipeline` まで通す。E2E PNG は重なり・bound 遷移を確認し、golden 更新前に差分を目視する。