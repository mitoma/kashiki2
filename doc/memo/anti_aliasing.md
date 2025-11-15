# kashiki のアンチエイリアシングの実装についての検討

## 基本方針

Analytical Anti-Aliasing (AAA) を採用し、MSAA や SSAA のようなサンプリングベースのアンチエイリアシングは補助的な利用にとどめる。

Analytical Anti-Aliasing
https://blog.frost.kiwi/analytical-anti-aliasing/

## 対応すべきエッジ

kashiki ではすべての線を直線またはベジェ曲線で表現している。
したがって、アンチエイリアシングを行うべきエッジは以下の 2 種類に分類できる。

1. 直線エッジ
2. ベジェ曲線エッジ

アンチエイリアシングは、直線エッジは原点と線分での三角を成したうちの線分側で行う。
ベジェ曲線エッジは、開始点、制御点、終了点で成す三角の内部で必要となる。
(つまり、全ての辺でアンチエイリアシングを行ってはいけない)

## 実装方法

頂点には x, y, z の座標情報に加えていくつかのアトリビュートを持たせることでエッジの種類を区別する。

原点 (0.0, 0.0, 0.0)

ベジエ曲線(曲線本体。アンチエイリアシングを行う)
始点   (1.0, 0.0, (Flip/Flop))
終点   (1.0, 0.0, (Flip/Flop))
制御点 (1.0, 1.0, 0.0)

ベジエ曲線(曲線の補完部。アンチエイリアシングを行わない)
原点   (0.0, 0.0, 0.0)
始点   (1.0, 1.0, 0.0)
終点   (1.0, 1.0, 0.0)

直線(直線本体。アンチエイリアシングを行う)
始点   (1.0, 0.0, (Flip/Flop))
終点   (1.0, 0.0, (Flip/Flop))

直線(直線の補完部。アンチエイリアシングを行わない)
始点   (0.0, 0.0, 0.0)
終点   (1.0, 0.0, (Flip/Flop))

## メモ

実装案としていくつかある。一つ目は Conservative Rasterization を利用したレンダリングと、通常のレンダリングを重ね合わせる方法。
もう一つは Conservative Rasterization の中で三角形の内側か外側化を判定してアンチエイリアシングを行う方法。

## 参考

[Conservative Rasterization Example](https://github.com/gfx-rs/wgpu/tree/trunk/examples/features/src/conservative_raster)
wgpu で Conservative Rasterization を利用する例。

[Perfect Anti-Aliasing](https://github.com/andrewlowndes/perfect-antialiasing)
アンチエイリアシングを Conservative Rasterization を踏まえて実装している例。