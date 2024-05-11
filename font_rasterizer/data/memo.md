# 炊 kashiki2

kashiki とは GPU ベースのテキストエディタを作るプロジェクトです。
[前身](https://mitoma-ryo.hatenadiary.org/entry/20140406/1396794851) は Java で書かれていました。
現在は改めて Rust と WebGPU(wgpu) でのリライト中です💪

## kashiki が頓挫した理由

- 😢 JOGL(Java の OpenGL Binding)の更新が止まった
- ❓ OpenGL や 3D プログラミングの知識不足
- 💔 テクスチャでフォントを描画すると拡大時の品質が微妙だった

## なぜ GPU ベースにするのか

(壮大なポエムを書く必要があります)

## 実装中の機能

- 📚 Editor の基本操作
- ✍ Font Rasterizer
- 〰️ Easing Function
- ⌨ Emacs Keybind
  - テキストエディタを快適に操作するにはキーバインドが欠かせません。
    また、キーバインドは Emacs を使うのが最良です。

## ドロイド君絵文字

https://github.com/aosp-mirror/platform_frameworks_base/tree/android-cts-4.4_r4/data/fonts