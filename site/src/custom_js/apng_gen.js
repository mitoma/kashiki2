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
                fontObject: font, // blob は後で取得
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
    let selectedFontIndex = null;
    let fonts = [];

    // フォント選択のドロップダウン初期化
    loadLocalFonts().then((loadedFonts) => {
        fonts = loadedFonts;
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
        fontSelect.addEventListener("change", (e) => {
            const selectedIndex = parseInt(e.target.value);
            if (isNaN(selectedIndex)) {
                selectedFontIndex = null;
                console.log("Using embedded fonts");
            } else {
                selectedFontIndex = selectedIndex;
                console.log(`Selected font: ${fonts[selectedIndex].fullName}`);
            }
        });
    });

    generateButton.addEventListener("click", async () => {
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

        // generateButton を押したときに選択されたフォントの blob を取得
        let selectedFontBinary = null;
        if (selectedFontIndex !== null && fonts[selectedFontIndex]) {
            const blob = await fonts[selectedFontIndex].fontObject.blob();
            selectedFontBinary = await blobToUint8Array(blob);
            console.log(
                `Loading font binary: ${fonts[selectedFontIndex].fullName}, size: ${selectedFontBinary.length} bytes`
            );
        }

        // 背景画像の blob を取得
        let backgroundImageBinary = null;
        const backgroundImageInput = document.getElementById("background-image");
        if (backgroundImageInput.files && backgroundImageInput.files[0]) {
            const imageFile = backgroundImageInput.files[0];
            backgroundImageBinary = await blobToUint8Array(imageFile);
            console.log(
                `Loading background image: ${imageFile.name}, size: ${backgroundImageBinary.length} bytes`
            );
        }

        apng.run_wasm(
            message.value,
            selectedSize,
            selectedTheme,
            selectedMotionType,
            fpsNum,
            transparentBg,
            selectedFontBinary,
            backgroundImageBinary
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

