import { invoke } from "@tauri-apps/api/core";

interface Settings {
  notesDir: string;
  notesDirIsDefault: boolean;
  autostart: boolean;
}

const notesDir = document.querySelector<HTMLInputElement>("#notes-dir")!;
const pickButton = document.querySelector<HTMLButtonElement>("#pick-notes-dir")!;
const resetButton = document.querySelector<HTMLButtonElement>("#reset-notes-dir")!;
const autostart = document.querySelector<HTMLInputElement>("#autostart")!;
const error = document.querySelector<HTMLParagraphElement>("#error")!;

function render(settings: Settings) {
  notesDir.value = settings.notesDir;
  notesDir.title = settings.notesDir;
  resetButton.disabled = settings.notesDirIsDefault;
  autostart.checked = settings.autostart;
}

async function run(command: string, args?: Record<string, unknown>) {
  error.hidden = true;
  try {
    render(await invoke<Settings>(command, args));
  } catch (err) {
    error.textContent = String(err);
    error.hidden = false;
  }
}

pickButton.addEventListener("click", () => void run("pick_notes_dir"));
resetButton.addEventListener("click", () => void run("reset_notes_dir"));
autostart.addEventListener("change", () =>
  void run("set_autostart", { enabled: autostart.checked }),
);

void run("get_settings");
