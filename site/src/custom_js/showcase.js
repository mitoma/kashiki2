import init, { send_log, toggle_direction, look_current_and_centering, change_theme_dark, change_theme_light, zoom_in, zoom_out, toggle_psychedelic } from "../wasm/showcase/showcase.js";
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
    document.getElementById("zoom-in-button").addEventListener("click", () => {
        zoom_in();
    });
    document.getElementById("zoom-out-button").addEventListener("click", () => {
        zoom_out();
    });
    document.getElementById("psychedelic-mode-button").addEventListener("click", () => {
        toggle_psychedelic();
    });
});
