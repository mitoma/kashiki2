import init, * as oogiri from "../wasm/oogiri_gen/oogiri_gen.js";
init().then(() => {
    let result = oogiri.run_wasm("お題をここに入力してください", "800x600", "Dark", "Smooth");
    const blob = new Blob([result], { type: "image/apng" });
    const url = URL.createObjectURL(blob);
    // Display the generated image
    const img = document.createElement("img");
    img.src = url;
    document.body.appendChild(img);
});
