# 文字アニメーション画像ジェネレーター

| コマンド                           | 説明                       |
| ---------------------------------- | -------------------------- |
| `<tate>`, `<vert>`, `<vertical>`   | 縦書きモードに切り替え     |
| `<yoko>`, `<hori>`, `<horizontal>` | 横書きモードに切り替え     |
| `<bs>` , `<backspace>`             | 1文字削除                  |
| `<enter>`, `<return>`              | 改行                       |
| `<wait-X>`                         | Xミリ秒待機(Xは任意の整数) |

以下のテキストエリアに文章を入力し、各種設定を選択して「Generate」ボタンを押すと Animation PNG が生成されます。

<style>
/* レイアウト */
.apng-gen { display: grid; grid-template-columns: 1fr 340px; gap: 16px; margin-top: 12px; }
@media (max-width: 980px) { .apng-gen { grid-template-columns: 1fr; } }
.apng-card { border: 1px solid rgba(128,128,128,.35); border-radius: 8px; padding: 12px; background: rgba(127,127,127,.06); }
.apng-card h3 { margin: 0 0 8px; font-size: 1rem; }

/* 入力系 */
.apng-controls .row { display: grid; grid-template-columns: 1fr; gap: 8px; margin-bottom: 10px; }
.apng-controls label { font-size: .9rem; opacity: .9; }
.apng-controls select,
.apng-controls input[type="number"],
.apng-controls input[type="file"],
.apng-controls input[type="range"],
.apng-controls input[type="button"],
.apng-controls button { width: 100%; }

#message { width: 100%; min-height: 10rem; }
#output { margin-top: 12px; }
.fps-inline { display: grid; grid-template-columns: 1fr 80px; gap: 8px; align-items: center; }
.toggle-row { display: inline-flex; gap: 8px; align-items: center; }
#background-preview { width: 100%; height: auto; border-radius: 6px; display: none; margin-top: 6px; }
.apng-actions { display: flex; gap: 8px; }
#generate-button { padding: .6rem 1rem; font-weight: 600; }
</style>

<script type="module" src="./custom_js/apng_gen.js"></script>

<div class="apng-gen">
    <div class="apng-card">
        <h3>Text</h3>
        <textarea id="message" cols="100" rows="10">
&lt;tate&gt;あしびきの
やまどり&lt;wait-500&gt;&lt;bs&gt;&lt;bs&gt;&lt;bs&gt;&lt;bs&gt;山鳥の尾の
しだり尾の
&lt;yoko&gt;ながながし夜を
ひとりかも寝む
        </textarea>
    </div>
    <div class="apng-card apng-controls">
        <h3>Settings</h3>
        <div class="row">
            <label for="font-select">Font</label>
            <select id="font-select"></select>
            <button id="enable-local-fonts" type="button" style="margin-top:6px;">Enable Local Fonts</button>
            <small id="local-fonts-status" style="display:block;opacity:.8;margin-top:4px;"></small>
        </div>
        <div class="row">
            <label for="image-size">Canvas & Ratio</label>
            <select id="image-size">
                <option value="square">Square</option>
                <option value="square4x3">Square 4:3</option>
                <option value="square3x4">Square 3:4</option>
                <option value="wide16x9">Wide 16:9</option>
                <option value="wide9x16">Wide 9:16</option>
            </select>
        </div>
        <div class="row">
            <label for="theme-select">Theme</label>
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
        </div>
        <div class="row">
            <label for="motion-type">Motion</label>
            <select id="motion-type">
                <option value="default">Default</option>
                <option value="poppy">Poppy</option>
                <option value="cool">Cool</option>
                <option value="energetic">Energetic</option>
                <option value="gentle">Gentle</option>
                <option value="minimal">Minimal</option>
            </select>
        </div>
        <div class="row">
            <label for="fps">FPS</label>
            <div class="fps-inline">
                <input id="fps-range" type="range" value="24" min="1" max="120" />
                <input id="fps" type="number" value="24" min="1" max="120" />
            </div>
        </div>
        <div class="row">
            <span class="toggle-row">
                <input id="transparent-bg" type="checkbox" />
                <label for="transparent-bg">Transparent Background</label>
            </span>
        </div>
        <div class="row">
            <label for="background-image">Background Image</label>
            <input id="background-image" type="file" accept="image/*" />
            <img id="background-preview" alt="Background preview" />
        </div>
        <div class="row apng-actions">
            <input id="generate-button" type="button" value="Generate" />
        </div>
    </div>
    <div class="apng-card" style="grid-column: 1 / -1;">
        <div id="progress"></div>
        <div id="output"></div>
    </div>
</div>

<!-- 軽量な UI 支援スクリプト（既存の apng_gen.js を崩さない） -->
<script>
(function () {
    const file = document.getElementById('background-image');
    const preview = document.getElementById('background-preview');
    if (file && preview) {
        file.addEventListener('change', () => {
            const f = file.files && file.files[0];
            if (f) {
                const url = URL.createObjectURL(f);
                preview.src = url;
                preview.style.display = 'block';
            } else {
                preview.removeAttribute('src');
                preview.style.display = 'none';
            }
        });
    }

    const fps = document.getElementById('fps');
    const fpsRange = document.getElementById('fps-range');
    if (fps && fpsRange) {
        const clamp = (v) => {
            const min = parseInt(fps.min || '1', 10);
            const max = parseInt(fps.max || '120', 10);
            v = Math.round(parseFloat(v || '24'));
            return Math.max(min, Math.min(max, v));
        };
        fpsRange.addEventListener('input', () => { fps.value = fpsRange.value; });
        fps.addEventListener('input', () => { fpsRange.value = clamp(fps.value); });
        fpsRange.value = fps.value;
    }
})();
</script>
