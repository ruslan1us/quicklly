import { invoke } from "@tauri-apps/api/core";
import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";

interface Settings {
  notesDir: string;
  notesDirIsDefault: boolean;
  mode: "daily" | "inbox";
  hotkey: string;
  hotkeyIsDefault: boolean;
  autostart: boolean;
}

const hotkey = document.querySelector<HTMLInputElement>("#hotkey")!;
const resetHotkeyButton = document.querySelector<HTMLButtonElement>("#reset-hotkey")!;
const notesDir = document.querySelector<HTMLInputElement>("#notes-dir")!;
const pickButton = document.querySelector<HTMLButtonElement>("#pick-notes-dir")!;
const resetButton = document.querySelector<HTMLButtonElement>("#reset-notes-dir")!;
const modeInputs = document.querySelectorAll<HTMLInputElement>('input[name="mode"]');
const autostart = document.querySelector<HTMLInputElement>("#autostart")!;
const appWindow = getCurrentWindow();
const error = document.querySelector<HTMLParagraphElement>("#error")!;

const MODIFIER_CODES = new Set([
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "ShiftLeft",
  "ShiftRight",
  "MetaLeft",
  "MetaRight",
]);

let currentHotkey = "";

function render(settings: Settings) {
  currentHotkey = settings.hotkey;
  if (document.activeElement !== hotkey) hotkey.value = settings.hotkey;
  resetHotkeyButton.disabled = settings.hotkeyIsDefault;
  notesDir.value = settings.notesDir;
  notesDir.title = settings.notesDir;
  resetButton.disabled = settings.notesDirIsDefault;
  modeInputs.forEach((input) => (input.checked = input.value === settings.mode));
  autostart.checked = settings.autostart;
}

/** Fits the window height to the page content. */
async function fitToContent() {
  await appWindow.setSize(new LogicalSize(window.innerWidth, document.body.scrollHeight));
}

async function run(command: string, args?: Record<string, unknown>) {
  error.hidden = true;
  try {
    render(await invoke<Settings>(command, args));
  } catch (err) {
    error.textContent = String(err);
    error.hidden = false;
  }
  // An error message changes the height.
  await fitToContent();
}

// Hotkey recording: focus the field and press any key or combination; clicking away cancels.
function modifiers(e: KeyboardEvent): string[] {
  const mods = [];
  if (e.ctrlKey) mods.push("Ctrl");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push("Super");
  return mods;
}

hotkey.addEventListener("focus", () => {
  hotkey.classList.add("recording");
  hotkey.value = "Press a key or combination…";
});

hotkey.addEventListener("blur", () => {
  hotkey.classList.remove("recording");
  hotkey.value = currentHotkey;
});

hotkey.addEventListener("keydown", (e) => {
  e.preventDefault();
  const mods = modifiers(e);
  if (MODIFIER_CODES.has(e.code)) {
    hotkey.value = [...mods.map((m) => (m === "Super" ? "Win" : m)), "…"].join("+");
    return;
  }
  hotkey.blur();
  void run("set_hotkey", { hotkey: [...mods, e.code].join("+") });
});

resetHotkeyButton.addEventListener("click", () => void run("reset_hotkey"));
pickButton.addEventListener("click", () => void run("pick_notes_dir"));
resetButton.addEventListener("click", () => void run("reset_notes_dir"));
modeInputs.forEach((input) =>
  input.addEventListener("change", () => void run("set_mode", { mode: input.value })),
);
autostart.addEventListener("change", () =>
  void run("set_autostart", { enabled: autostart.checked }),
);

// The window is created hidden: size it to the content first, then show it.
void run("get_settings").then(async () => {
  await appWindow.center();
  await appWindow.show();
  await appWindow.setFocus();
});
