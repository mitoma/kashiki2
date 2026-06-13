const status = document.getElementById("pig-action-game-status");

const setStatus = (message) => {
    if (status) {
        status.textContent = message;
    }
};

const load = async () => {
    try {
        const wasm = await import("../wasm/pig_action_game/pig_action_game.js");
        await wasm.default();
        setStatus("Ready. Click the canvas and use arrow keys / space.");
    } catch (error) {
        console.error(error);
        setStatus(
            "Failed to load pig_action_game WASM. Run build-sites.sh or generate site/src/wasm/pig_action_game first.",
        );
    }
};

load();