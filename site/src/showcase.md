# ShowCase

Technical showcase.
This demo use WASM & WebGL.

Click area and edit it!
<div id="kashikishi-area"></div>

<script type="module">
  import init from "./wasm/showcase/showcase.js";
  init().then(() => {
    console.log("WASM Loaded");
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