import { invoke } from "@tauri-apps/api/core";
import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";

interface Settings {
  notesDir: string;
  notesDirIsDefault: boolean;
  mode: "daily" | "inbox";
  autostart: boolean;
}

const notesDir = document.querySelector<HTMLInputElement>("#notes-dir")!;
const pickButton = document.querySelector<HTMLButtonElement>("#pick-notes-dir")!;
const resetButton = document.querySelector<HTMLButtonElement>("#reset-notes-dir")!;
const modeInputs = document.querySelectorAll<HTMLInputElement>('input[name="mode"]');
const autostart = document.querySelector<HTMLInputElement>("#autostart")!;
const appWindow = getCurrentWindow();
const error = document.querySelector<HTMLParagraphElement>("#error")!;

function render(settings: Settings) {
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
