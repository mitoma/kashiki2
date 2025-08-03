# シェーダーのテスト


```bash
# シェーダーのテストをビルド
cargo build --example aa_test --release


# シェーダーのデバッグを有効にする

# cmd.exe で有効化
set FONT_RASTERIZER_DEBUG_SHADER="true"
# cmd.exe で無効化
set FONT_RASTERIZER_DEBUG_SHADER=""

# PowerShell で有効化
$Env:FONT_RASTERIZER_DEBUG_SHADER = "true"
# PowerShell で無効化
$Env:FONT_RASTERIZER_DEBUG_SHADER = ""

# シェーダーのテストを実行
.\target\release\examples\aa_test.exe -c あ
```
