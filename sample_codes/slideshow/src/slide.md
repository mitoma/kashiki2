Font
Rasterizer
on WebGPU

Font Rasterizer を
WebGPU で実装しました。

Font Rasterizer
(フォントラスタライザ)
とは

フォントの字形を
ベクターからビットマップ化
する処理を指します

これを GPU 上で行うことで
高詳細なフォントをリアルタイムに
アニメーションさせることが可能になります

このデモでは
TrueType/OpenType フォントを
GPU 上でレンダリングしています

テクスチャや SDF で描画するのではなく
GPU でベクターをラスタライズしているため
複雑なグリフを拡大しても破綻しません

例をどうぞ

憂鬱

いかがでしょう

このデモでは
スーパーサンプリングによる
アンチエイリアスも
実装されています

このゆらゆらとした
動きは VertexShader で
文字の各頂点を動かすことで
実現しています

絵文字も対応しています

🐢🐖🐕
🍣☺🌀

カラーでは
ありません😞

すべての文字が
三次元空間上に
配置されているため
横目で見ることも
できます😒

鏡文字で
読みたければ
裏返すだけで
済みます

以上、開発中の
Font Rasterizer の
紹介を終わります
