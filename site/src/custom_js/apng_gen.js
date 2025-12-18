import init, * as apng from "../wasm/apng_gen/apng_gen.js";

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
    const enableFontsBtn = document.getElementById("enable-local-fonts");
    const localFontsStatus = document.getElementById("local-fonts-status");
    let selectedFontIndex = null;
    let fonts = [];

    // まずは埋め込みフォントのみ選択可能にしておく
    const defaultOption = document.createElement("option");
    defaultOption.value = "";
    defaultOption.textContent = "No local font (use embedded)";
    fontSelect.appendChild(defaultOption);

    // フォント選択の変更イベント（常時アタッチ）
    fontSelect.addEventListener("change", (e) => {
        const selectedIndex = parseInt(e.target.value);
        if (isNaN(selectedIndex)) {
            selectedFontIndex = null;
            console.log("Using embedded fonts");
        } else {
            selectedFontIndex = selectedIndex;
            if (fonts[selectedIndex]) {
                console.log(`Selected font: ${fonts[selectedIndex].fullName}`);
            }
        }
    });

    // クリック（ユーザー操作）でローカルフォントアクセスを要求し、選択肢に追加
    async function populateLocalFontsViaClick() {
        if (!("queryLocalFonts" in window)) {
            console.warn("Local Font Access API is not supported in this browser");
            if (localFontsStatus) {
                localFontsStatus.textContent = "Local Font Access API is not supported in this browser.";
            }
            return;
        }

        try {
            if (localFontsStatus) localFontsStatus.textContent = "Requesting access to local fonts...";

            // ユーザー操作内で実行することでパーミッションプロンプトを表示可能にする
            const loadedFonts = await loadLocalFonts();
            fonts = loadedFonts || [];

            // 既存のローカルフォント候補を一旦クリア（先頭の埋め込みオプションは保持）
            while (fontSelect.options.length > 1) {
                fontSelect.remove(1);
            }

            if (fonts.length === 0) {
                if (localFontsStatus) localFontsStatus.textContent = "No local fonts available or permission denied.";
            } else {
                fonts.forEach((font, index) => {
                    const option = document.createElement("option");
                    option.value = index.toString();
                    option.textContent = font.fullName;
                    fontSelect.appendChild(option);
                });
                if (localFontsStatus) localFontsStatus.textContent = "Local fonts enabled.";
                if (enableFontsBtn) {
                    enableFontsBtn.disabled = true;
                    enableFontsBtn.textContent = "Local Fonts Enabled";
                }
            }
        } catch (err) {
            console.error("Error while enabling local fonts:", err);
            if (localFontsStatus) localFontsStatus.textContent = "Failed to enable local fonts.";
        }
    }

    if (enableFontsBtn) {
        enableFontsBtn.addEventListener("click", populateLocalFontsViaClick);
        // 対応可否を表示
        if (!("queryLocalFonts" in window)) {
            enableFontsBtn.disabled = true;
            if (localFontsStatus) {
                localFontsStatus.textContent = "Local Font Access API is not supported in this browser.";
            }
        }
    }

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

