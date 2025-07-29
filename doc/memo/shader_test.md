# シェーダーのテスト


```bash
# シェーダーのテストをビルド
cargo build --example aa_test --release


# シェーダーのデバッグを有効にする
set FONT_RASTERIZER_DEBUG_SHADER=true

# cmd.exe で unset の代わりに環境変数を削除する方法は？
set FONT_RASTERIZER_DEBUG_SHADER=""

# シェーダーのテストを実行
.\target\release\examples\aa_test.exe -c あ
```
