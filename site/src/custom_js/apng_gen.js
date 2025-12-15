import init, * as apng from "../wasm/apng_gen/apng_gen.js";
console.log("hoihoi");

// ローカルフォント一覧を取得
async function loadLocalFonts() {
    if (!("queryLocalFonts" in window)) {
        console.warn("Local Font Access API is not supported in this browser");
        return [];
    }

    try {
        const availableFonts = await window.queryLocalFonts();
        const fonts = [];
        for (const font of availableFonts) {
            fonts.push({
                family: font.family,
                fullName: font.fullName || font.family,
                postscriptName: font.postscriptName,
                blob: await font.blob(),
            });
        }
        return fonts;
    } catch (err) {
        console.error("Error loading local fonts:", err);
        return [];
    }
}

// Blob を Uint8Array に変換
async function blobToUint8Array(blob) {
    const arrayBuffer = await blob.arrayBuffer();
    return new Uint8Array(arrayBuffer);
}

init().then(async () => {
    const generateButton = document.getElementById("generate-button");
    const fontSelect = document.getElementById("font-select");
    let selectedFontBinary = null;

    // フォント選択のドロップダウン初期化
    loadLocalFonts().then((fonts) => {
        // "No local font" オプションを追加
        const option = document.createElement("option");
        option.value = "";
        option.textContent = "No local font (use embedded)";
        fontSelect.appendChild(option);

        // 取得したフォントをオプションに追加
        fonts.forEach((font, index) => {
            const option = document.createElement("option");
            option.value = index.toString();
            option.textContent = font.fullName;
            fontSelect.appendChild(option);
        });

        // フォント選択の変更イベント
        fontSelect.addEventListener("change", async (e) => {
            const selectedIndex = parseInt(e.target.value);
            if (isNaN(selectedIndex)) {
                selectedFontBinary = null;
                console.log("Using embedded fonts");
            } else {
                selectedFontBinary = await blobToUint8Array(fonts[selectedIndex].blob);
                console.log(
                    `Selected font: ${fonts[selectedIndex].fullName}, size: ${selectedFontBinary.length} bytes`
                );
            }
        });
    });

    generateButton.addEventListener("click", () => {
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
        apng.run_wasm(
            message.value,
            selectedSize,
            selectedTheme,
            selectedMotionType,
            fpsNum,
            transparentBg,
            selectedFontBinary
        ).then((res) => {
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

