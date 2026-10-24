---
title: Anti-Aliasing Strategy
kind: decision
status: production
updated: 2026-10-11
source_refs:
  - ../../../font_rasterizer/src/windfoil.rs
  - ../../../font_rasterizer/src/shader/windfoil.wgsl
  - ../../../font_rasterizer/src/vector_vertex.rs
  - https://github.com/texel-org/windfoil-algorithm/blob/main/docs/ALGORITHM.md
  - ../../memo/anti_aliasing.md
  - ../../../memories/repo/vector_vertex_aa_analysis.md
  - ../../../memories/repo/font_overlap_artifact_notes.md
  - https://blog.frost.kiwi/analytical-anti-aliasing/
related_pages:
  - ../concepts/text-rendering-pipeline.md
  - ./overlap-removal-strategy.md
  - ../sources/source-anti-aliasing.md
  - ../sources/source-frost-analytical-anti-aliasing.md
  - ../sources/source-vector-vertex-aa-analysis.md
  - ../sources/source-font-overlap-artifact-notes.md
  - ../sources/source-vector-vertex-builder.md
  - ../sources/source-overlap-shader.md
  - ../sources/source-outline-shader.md
---

# Anti-Aliasing Strategy

## 現行方針

- Windfoil の解析的な巻き数積分を採用する。`Quarity` による出力解像度の変更は従来どおり残す
- Builder は有向二次曲線を直接生成し、直線も制御点を中点にした同じ構造で扱う。頂点種別や補助三角形は不要
- 曲線と水平行バンドを storage buffer に格納し、局所ピクセル範囲で曲線積分を評価する
- non-zero は積分の絶対値を 1 に制限し、even-odd は周期 2 の三角波で被覆率へ折り畳む
- 座標依存モーションは元の制御点を変形し、画面座標で再分割・積分する
- パス・巻き数のテクスチャは使わない。色合成用テクスチャと straight-alpha 出力への変換は残す
- AA 無効時は中心点の巻き数を評価する

## この判断の理由

- GPU 上で文字形状を高精度に扱いたい
- ベクトル形状と相性が良い
- 曲線積分と行バンドを基本単位にすれば、旧三角形表現を作ってから曲線を取り出す二重の変換を避けられる
- 中心点探索や補助頂点の重複対策は旧方式のためのコストであり、Windfoil のモデルには不要である

## historical な知見

- 凸性タグ付けで fill coverage を救済する案は、輪郭間の巻き相殺を壊すため棄却されている
- ベジエ接続部の AA 漏れに対しては、弦側を除外した signed accumulation と edge 寄与数の平均化を production / debug の両方へ反映した
- 過去に debug shader で試した MAX coverage ブレンドは、現行の平均化と fill rule の構造が異なるため採用しない
- 旧方式では `overlap_shader.wgsl` が `front_facing` による符号付き winding を積算し、`outline_shader.wgsl` が count texture から alpha を復元していた。現行の Windfoil はこの経路を使用しない

## overlap remover との関係

- even-odd ベースの経路では、font 側で重複除去して見た目を揃える戦略が併用されてきた
- 現行の non-zero は曲線の向きによる積分を使う。既存のフォント前処理としての overlap remover は今回変更せず、除去の可否は別途品質検証する

## 保留中の論点

- 巻き数の重複・交差をまたぐピクセルで、単一の積分値の折り畳みが真の塗り面積と一致しない場合がある
- 通常経路の回転・せん断は `fwidth` の軸平行な局所ボックス近似であり、制御点の透視投影も近似である
- 三次曲線の二次近似、ほぼ直線の簡略化、f32 精度の制限は残る
- 座標依存モーション経路と極小サイズの性能を実測する。速度向上は保証しない
- overlap remover の不要化は、独立した品質検証の後に判断する

## 反映済み source

- `font_rasterizer/src/vector_vertex.rs`
- `font_rasterizer/src/windfoil.rs`
- `font_rasterizer/src/shader/windfoil.wgsl`

## 次に深掘りすべき source

- `font_rasterizer/src/rasterizer_renderrer.rs`
- `font_rasterizer/src/vector_vertex_png_renderer.rs` の GPU 被覆率回帰テスト