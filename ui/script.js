// Use the global Tauri API
const invoke = window.__TAURI__.core.invoke;

document.addEventListener('DOMContentLoaded', () => {
    document.getElementById("ping").addEventListener("click", async () => {
        const msg = await invoke("say_hello", { name: "friend" });
        const out = document.getElementById("out");
        out.textContent = msg;
        out.classList.remove("hidden");
        await invoke("my_custom_command");
    });
});
