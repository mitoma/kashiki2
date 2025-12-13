# 文字アニメーション画像ジェネレーター

| コマンド                           | 説明                       |
| ---------------------------------- | -------------------------- |
| `<tate>`, `<vert>`, `<vertical>`   | 縦書きモードに切り替え     |
| `<yoko>`, `<hori>`, `<horizontal>` | 横書きモードに切り替え     |
| `<bs>` , `<backspace>`             | 1文字削除                  |
| `<enter>`, `<return>`              | 改行                       |
| `<wait-X>`                         | Xミリ秒待機(Xは任意の整数) |

以下のテキストエリアに文章を入力し、各種設定を選択して「Generate」ボタンを押すと Animation PNG が生成されます。

<script type="module" src="./custom_js/oogiri_gen.js"></script>
<textarea id="message" cols="100" rows="10">
&lt;tate&gt;あしびきの
やまどり&lt;wait-500&gt;&lt;bs&gt;&lt;bs&gt;&lt;bs&gt;&lt;bs&gt;山鳥の尾の
しだり尾の
&lt;yoko&gt;ながながし夜を
ひとりかも寝む
</textarea>
<br/>
<select id="image-size">
    <option value="square">Square</option>
    <option value="square4x3">Square 4:3</option>
    <option value="square3x4">Square 3:4</option>
    <option value="wide16x9">Wide 16:9</option>
    <option value="wide9x16">Wide 9:16</option>
</select>
<select id="theme-select">
    <option value="solarized-dark">Solarized Dark</option>
    <option value="solarized-light">Solarized Light</option>
    <option value="high-contrast-dark">High Contrast Dark</option>
    <option value="high-contrast-light">High Contrast Light</option>
    <option value="warm-dark">Warm Dark</option>
    <option value="warm-light">Warm Light</option>
    <option value="cool-dark">Cool Dark</option>
    <option value="cool-light">Cool Light</option>
    <option value="vivid">Vivid</option>
</select>
<select id="motion-type">
    <option value="default">Default</option>
    <option value="poppy">Poppy</option>
    <option value="cool">Cool</option>
    <option value="energetic">Energetic</option>
    <option value="gentle">Gentle</option>
    <option value="minimal">Minimal</option>
</select>
<input id="fps" type="number" value="24" min="1" max="120"/>
<input id="generate-button" type="button" value="Generate"/>
<div id="progress"></div>
<div id="output"></div>
