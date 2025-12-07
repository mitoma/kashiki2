import init, * as oogiri from "../wasm/oogiri_gen/oogiri_gen.js";
console.log("hoihoi");

init().then(() => {
    console.log("WASM module initialized");
    oogiri.run_wasm("お題をここに入力してください", "", "", "").then((res) => {
        console.log("hello");
        const blob = new Blob([result], { type: "image/apng" });
        const url = URL.createObjectURL(blob);
        // Display the generated image
        const img = document.createElement("img");
        img.src = url;
        document.body.appendChild(img);
    });
});
