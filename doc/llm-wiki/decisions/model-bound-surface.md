- Card は border なしでは通常 bound を使う。border は bound から 1 セル外側が中心線で、線幅は viewBox 9 単位中 1 単位のため、surface inset は内縁の `17/18` セル。Rounded の内側曲線は `border_top_left.svg` の 4/9 セル半径を使い、border 形状に一致させる。
---
title: Model Bound Surface
kind: decision
status: production
updated: 2026-11-06
source_refs:
  - ../../ai-agent/plans/model-surface-background.md
  - ../../../ui_support/src/layout_engine/model.rs
  - ../../../ui_support/src/layout_engine/default_world.rs
  - ../../../font_rasterizer/src/rasterizer_pipeline.rs
related_pages:
  - ../concepts/text-rendering-pipeline.md
  - ../workflows/model-surface-e2e.md
  - ../sources/source-model-bound-surface.md
---

# Model Bound Surface

## 決定

モデル間の重なりは深度ではなく、World のモデル順で合成する。面だけを全モデル分先に描くと、後続の背面文字が手前モデルの面より上に描かれてしまうため、描画単位をモデル batch とする。

各モデルは surface と content（vector / glyph）を順序付き batch として渡す。Windfoil は全 batch を一回の render pass と resolve に集め、既存の premultiplied-alpha blend で順序を保つ。モデルごとの resolve と全画面 composite を省く。Legacy は coverage/count 解決が batch 間で混ざるため、従来の per-batch resolve/composite fallback を維持する。

## 現行仕様

- TextEdit は既定で `ThemeBackground` surface を持つ。SVG など、面を持たないモデルの既定値は `None`。
- 色は `ColorTheme::background()` に追従する。モデル API から `None`、テーマ背景、RGB 色を選べる。
- 面は現在のモデル位置・回転・world scale・bound を利用し、bound アニメーションに追従する。
- surface instance は TextEdit glyph と同じ `camera_detail` を使用する。Card overlay のように `IGNORE_CAMERA` を使う UI でも面と文字の座標系を揃える。
- Card は border なしでは通常 bound を使う。border ありでは内縁まで塗り、Rounded は bound に合わせて生成した半径 0.5 セルの rounded geometry を使う。単に四角い矩形を広げない。
- SVG geometry は RenderState 初期化時に一度登録する。利用側 callback に登録責務を置かない。
- modal は通常 World とは独立して合成し、既存の modal 背景ぼかしと最終合成を保つ。
- Windfoil では normal/modal それぞれ一回の ordered render/resolve にまとめる。Legacy は従来経路を使う。
- ordered resolve は従来の複数回 resolve と AA/量子化結果が少し異なるため、視覚確認後に該当 E2E PNG golden を更新する。
- 任意に交差する面の可視性や深度テストは対象外。交差の前後関係はモデル列順で定義する。

## 性能確認

同一 Windows debug 環境で command palette の `render 4: run all stage` は旧経路の約 20 ms/frame から約 1 ms/frame、tagbase memo は約 6 ms/frame から 1 ms 未満になった。これは GPU timestamp ではなく phase wall time の比較だが、per-batch resolve/composite を除いた効果が確認できた。Legacy の廃止は別判断とし、当面は fallback を残す。

## 拡張方針

角丸と border の境界共有では surface / border が共通の corner radius と bound を参照する。半透明は batch 順を保ち、coverage と opacity を分離して合成する。初期の矩形・不透明面の挙動へ深度規則を持ち込まない。

描画順と E2E 検証手順は [model-surface-e2e workflow](../workflows/model-surface-e2e.md) を参照。