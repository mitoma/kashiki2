# ShowCase

Technical showcase.
This demo use WASM & WebGL.

Click area and edit it!
<div id="kashikishi-area"></div>
<input id="toggle-direction-button" type="button" value="Toggle direction"></input>
<input id="centering-button" type="button" value="Fit the entire document"></input>
<input id="dark-mode-button" type="button" value="Dark mode"></input>
<input id="light-mode-button" type="button" value="Light mode"></input>

<script type="module">
  import init, { send_log, toggle_direction, look_current_and_centering, change_theme_dark, change_theme_light } from "./wasm/showcase/showcase.js";
  init().then(() => {
    console.log("WASM Loaded");
    send_log("Hello from JS");
    document.getElementById("toggle-direction-button").addEventListener("click", () => {
      toggle_direction();
    });
    document.getElementById("centering-button").addEventListener("click", () => {
      look_current_and_centering();
    });
    document.getElementById("dark-mode-button").addEventListener("click", () => {
      change_theme_dark();
    });
    document.getElementById("light-mode-button").addEventListener("click", () => {
      change_theme_light();
    });
  });
</script>

Usage
- Allow key : move curser.
- Ctrl + 0 : Reset zoom.
- Ctrl + 9: Fit the document's width to the screen.
- Ctrl + 8: Fit the document's height to the screen.
- Ctrl + Minus : Zoom out.
- Ctrl + Plus : Zoom in.
- Ctrl + Shift + L : Fit the entire document within the screen.
- Ctrl + Shift + D : Toggle direction. You can switch between vertical and horizontal writing.
- Ctrl + X, Ctrl + D, Ctrl + D : Change Theme. (Soralized Dark)
- Ctrl + X, Ctrl + D, Ctrl + L : Change Theme. (Soralized Light)
- Alt + ←→ : Adjust character spacing.
- Alt + ↑↓ : Adjust line spacing.
- Alt + Shift + ←→ : Adjust char width.
- Alt + Shift + ↑↓ : Adjust char height.
- Some Emacs keybindings can be used, but be careful as they may conflict with browser shortcuts and surprise you. (e.g., Ctrl + w closes the tab)