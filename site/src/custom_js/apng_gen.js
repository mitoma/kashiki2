import init, * as apng from "../wasm/apng_gen/apng_gen.js";
console.log("hoihoi");

init().then(() => {
    document.getElementById("generate-button").addEventListener("click", () => {
        console.log("WASM module initialized");
        const message = document.getElementById("message");
        const imageSizeSelect = document.getElementById("image-size");
        const selectedSize = imageSizeSelect.value;
        const themeSelect = document.getElementById("theme-select");
        const selectedTheme = themeSelect.value;
        const motionTypeSelect = document.getElementById("motion-type");
        const selectedMotionType = motionTypeSelect.value;
        const fps = document.getElementById("fps");
        const fpsNum = fps.value;
        const transparentBgCheckbox = document.getElementById("transparent-bg");
        const transparentBg = transparentBgCheckbox.checked ? true : false;
        apng.run_wasm(message.value, selectedSize, selectedTheme, selectedMotionType, fpsNum, transparentBg).then((res) => {
            const blob = new Blob([res], { type: "image/apng" });
            const url = URL.createObjectURL(blob);
            // Display the generated image
            const img = document.createElement("img");
            img.src = url;
            const output = document.getElementById("output");
            // Clear previous output
            output.innerHTML = "";
            output.appendChild(img);
        });
    });
});

