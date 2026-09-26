import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const input = document.querySelector<HTMLInputElement>("#note")!;
const appWindow = getCurrentWindow();

function clearError() {
  input.classList.remove("error");
  input.title = "";
}

async function hide() {
  input.value = "";
  clearError();
  await appWindow.hide();
}

async function save() {
  const text = input.value.trim();
  if (!text) {
    await hide();
    return;
  }
  if (text === "/config") {
    await invoke("open_settings");
    await hide();
    return;
  }
  try {
    await invoke("save_note", { text });
    await hide();
  } catch (err) {
    input.classList.add("error");
    input.title = String(err);
  }
}

input.addEventListener("keydown", (e) => {
  if (e.isComposing) return;
  if (e.key === "Enter") {
    e.preventDefault();
    void save();
  } else if (e.key === "Escape") {
    e.preventDefault();
    void hide();
  }
});

input.addEventListener("input", clearError);

// The window is hidden rather than destroyed, so refocus the input on every show.
window.addEventListener("focus", () => input.focus());
