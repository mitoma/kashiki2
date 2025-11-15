# kashiki のアンチエイリアシングの実装

## 基本方針

Analytical Anti-Aliasing (AAA) を採用し、`smoothstep` と `fwidth` を用いた距離場ベースのアンチエイリアシングを実装している。MSAA や SSAA のようなサンプリングベースのアンチエイリアシングは使用していない。

参考: [Analytical Anti-Aliasing](https://blog.frost.kiwi/analytical-anti-aliasing/)

## 対応すべきエッジ

kashiki ではすべての線を直線または2次ベジェ曲線で表現している。
したがって、アンチエイリアシングを行うべきエッジは以下の 2 種類に分類できる:

1. **直線エッジ**: 始点と終点を結ぶ線分
2. **ベジェ曲線エッジ**: 始点、制御点、終点で定義される2次ベジェ曲線

3次ベジェ曲線は `bezier_converter` クレートを使用して複数の2次ベジェ曲線に変換される。

## 頂点タイプの定義

各頂点には `vertex_type` というアトリビュートが付与され、シェーダー内で三角形の種類を判定する。

### vertex_type の値

```
原点B  : 0  (ベジェ曲線用の原点)
原点L  : 1  (直線用の原点)
始点B  : 2  (ベジェ曲線の始点)
始点L  : 3  (直線の始点)
終点B  : 4  (ベジェ曲線の終点)
終点L  : 5  (直線の終点)
制御点 : 6  (ベジェ曲線の制御点)
```

### 三角形の構成

**ベジェ曲線部 (曲線本体 - アンチエイリアシング対象)**
- 原点B (0) + 始点B (2) + 制御点 (6)
- 始点B (2) + 終点B (4) + 制御点 (6)

**ベジェ補助直線 (塗りつぶし用 - アンチエイリアシング対象外)**
- 原点B (0) + 始点B (2) + 終点B (4)

**直線部 (アンチエイリアシング対象)**
- 原点L (1) + 始点L (3) + 終点L (5)

## シェーダーでの実装

### 頂点シェーダー

各頂点タイプに応じて、以下の2つの属性を設定:

1. **wait (ウエイト値)**: フラグメントシェーダーでの距離計算に使用
   - `wait.x`: 原点または制御点からのウエイト
   - `wait.y`: 始点からのウエイト
   - `wait.z`: 終点からのウエイト

2. **triangle_type**: 三角形の種類を判定
   - `triangle_type.x == 1.0`: ベジェ曲線部
   - `triangle_type.y == 1.0`: ベジェ補助直線
   - `triangle_type.z == 1.0`: 直線部

### フラグメントシェーダー

距離場ベースのアンチエイリアシング処理:

**ベジェ曲線の場合:**
```wgsl
// SDF距離計算 (implicit equation: (u*0.5+v)^2 - v = 0)
let bezier_distance = pow((in.wait.x * 0.5 + in.wait.y), 2.0) - in.wait.y;
let bezier_distance_fwidth = fwidth(bezier_distance);
let alpha = 1.0 - smoothstep(-bezier_distance_fwidth / 2.0, 
                             bezier_distance_fwidth / 2.0, 
                             bezier_distance);
```

**直線の場合:**
```wgsl
// SDF距離計算 (原点からの距離)
let triangle_distance = in.wait.x;
let triangle_distance_fwidth = fwidth(triangle_distance);
let alpha = smoothstep(-triangle_distance_fwidth / 2.0, 
                       triangle_distance_fwidth / 2.0, 
                       triangle_distance);
```

### 重要な実装上の注意点

- **fwidth の実行タイミング**: WebGPU では分岐の前に `fwidth` を実行する必要がある。条件分岐内でのみ呼び出すと、一部の実装でエラーが発生する。
- **マルチターゲットレンダリング**: カラーとカウント情報を別々のテクスチャに出力し、後段で合成する。
- **エッジ判定**: `NEAR_ZERO` (1e-6) と `NEAR_ONE` (1.0 - 1e-6) を使用した浮動小数点比較で正確な判定を行う。

## ベジェ曲線の平坦性判定

ベジェ曲線を直線として近似できるかの判定は、現在は3次ベジェから2次ベジェへの変換時に `bezier_converter` クレート内で行われる。

将来的な改善案として、以下のような方法が検討できる:

1. **正規化距離による判定**: 制御点から直線への距離を正規化して判定
2. **曲率計算**: 解析的に曲率の最大値を計算して閾値と比較
3. **中点分割**: 再帰的な分割により平坦性を判定

## 参考資料

- [Analytical Anti-Aliasing](https://blog.frost.kiwi/analytical-anti-aliasing/)
- [Conservative Rasterization Example](https://github.com/gfx-rs/wgpu/tree/trunk/examples/features/src/conservative_raster) - wgpu での Conservative Rasterization の例
- [Perfect Anti-Aliasing](https://github.com/andrewlowndes/perfect-antialiasing) - Conservative Rasterization を用いた実装例
