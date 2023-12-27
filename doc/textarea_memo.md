editor と textarea をどのようにつなげるか。

- editor と textarea を一体化する
- editor に observer を用意して callback などで変更を textarea に伝える
  - editor に sender を差し込み変更をキャプチャする


BufferChar は (row, col, char) の tuple で文書中の位置 + 文字種の情報を保持している
GPU 上で描画するためには (y, x, glyph) の値に変更する必要がある。

- y は全体の行数に依存する
- x はその行の文字数に依存する
- glyph は tuple から決定できる(リガチャはスコープ外のため)

この場合、文書の最初を編集すると全体を再計算する必要がある

編集対象行のみが主要な依存と考えると別の考え方もできそうではあるが。



Buffer
 - Line
   - Char

Char