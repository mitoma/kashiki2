import init, * as kashiki from "../wasm/showcase/showcase.js";
init().then(() => {
    console.log("WASM Loaded");
    kashiki.send_log("Hello from JS");
    document.getElementById("toggle-direction-button").addEventListener("click", () => {
        kashiki.toggle_direction();
    });
    document.getElementById("centering-button").addEventListener("click", () => {
        kashiki.look_current_and_centering();
    });
    document.getElementById("dark-mode-button").addEventListener("click", () => {
        kashiki.change_theme_dark();
    });
    document.getElementById("light-mode-button").addEventListener("click", () => {
        kashiki.change_theme_light();
    });
    document.getElementById("zoom-in-button").addEventListener("click", () => {
        kashiki.zoom_in();
    });
    document.getElementById("zoom-out-button").addEventListener("click", () => {
        kashiki.zoom_out();
    });
    document.getElementById("psychedelic-mode-button").addEventListener("click", () => {
        kashiki.toggle_psychedelic();
    });
});
