# レイアウトエンジン

画面要素について整理する。

- World
  - 1 つの Camera と Feature のリストを持つ
  - 各 Component は World に配置される
  - 配置は Layout Engine に任される
- Paper
  - UI 要素の一つの単位を表す
  - 要素の中心やサイズなどを返すようにする
- Text
  - Paper に属する文字列

## コンテキストの持ち方について検討する

- Preference
  - font
  - color_theme
  - glyph_vertex_buffer
- WorldContext
- ModelContext
  - add motion
  - update motion
  - remove motion
  - 

