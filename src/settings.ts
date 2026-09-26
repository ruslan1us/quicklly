import { invoke } from "@tauri-apps/api/core";

interface Settings {
  notesDir: string;
  notesDirIsDefault: boolean;
}

const notesDir = document.querySelector<HTMLInputElement>("#notes-dir")!;
const pickButton = document.querySelector<HTMLButtonElement>("#pick-notes-dir")!;
const resetButton = document.querySelector<HTMLButtonElement>("#reset-notes-dir")!;
const error = document.querySelector<HTMLParagraphElement>("#error")!;

function render(settings: Settings) {
  notesDir.value = settings.notesDir;
  notesDir.title = settings.notesDir;
  resetButton.disabled = settings.notesDirIsDefault;
}

async function run(command: string) {
  error.hidden = true;
  try {
    render(await invoke<Settings>(command));
  } catch (err) {
    error.textContent = String(err);
    error.hidden = false;
  }
}

pickButton.addEventListener("click", () => void run("pick_notes_dir"));
resetButton.addEventListener("click", () => void run("reset_notes_dir"));

void run("get_settings");
