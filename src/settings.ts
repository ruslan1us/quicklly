import { invoke } from "@tauri-apps/api/core";
import { appWindow, fitToContent, reveal } from "./popup";
import { type Theme, applyTheme } from "./theme";

interface Settings {
  notesDir: string;
  notesDirIsDefault: boolean;
  mode: "daily" | "inbox";
  hotkey: string;
  hotkeyIsDefault: boolean;
  theme: Theme;
  autostart: boolean;
}

/** One line of the settings list. */
interface Row {
  label: string;
  value(s: Settings): string;
  /** ←/→ (and Enter): switch to the previous/next value. */
  cycle?(s: Settings, step: 1 | -1): void;
  /** Enter: edit the value. */
  edit?(): void;
  /** Del: back to the default. */
  reset?(): void;
}

const rowsList = document.querySelector<HTMLUListElement>("#rows")!;
const error = document.querySelector<HTMLParagraphElement>("#error")!;
const help = document.querySelector<HTMLElement>("#help")!;

const HELP = "↑↓ select · ←→ change · Enter edit · Del reset · Esc close";
const RECORDING_HELP = "Press a key or combination · Esc cancel";
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

let settings: Settings | null = null;
let selected = 0;
/** While recording a hotkey: the modifiers held so far, e.g. "Ctrl+Alt+…". */
let recording: string | null = null;

const choice = (text: string) => `‹ ${text} ›`;

const THEMES: { theme: Theme; name: string }[] = [
  { theme: "default", name: "Default" },
  { theme: "defaultPlus", name: "Default+" },
  { theme: "light", name: "Light" },
];
const themeIndex = (s: Settings) => THEMES.findIndex((t) => t.theme === s.theme);

const rows: Row[] = [
  {
    label: "Hotkey",
    value: (s) => recording ?? s.hotkey,
    edit: () => {
      recording = "Press a key or combination…";
      render();
    },
    reset: () => void run("reset_hotkey"),
  },
  {
    label: "Notes folder",
    value: (s) => s.notesDir,
    edit: () => void run("pick_notes_dir"),
    reset: () => void run("reset_notes_dir"),
  },
  {
    label: "Notes file",
    value: (s) => choice(s.mode === "daily" ? "One file per day" : "Single inbox"),
    cycle: (s) => void run("set_mode", { mode: s.mode === "daily" ? "inbox" : "daily" }),
    reset: () => void run("set_mode", { mode: "daily" }),
  },
  {
    label: "Theme",
    value: (s) => choice(THEMES[themeIndex(s)]?.name ?? s.theme),
    cycle: (s, step) => {
      const next = THEMES[(themeIndex(s) + step + THEMES.length) % THEMES.length];
      void run("set_theme", { theme: next.theme });
    },
    reset: () => void run("set_theme", { theme: "default" }),
  },
  {
    label: "Start with Windows",
    value: (s) => choice(s.autostart ? "On" : "Off"),
    cycle: (s) => void run("set_autostart", { enabled: !s.autostart }),
    reset: () => void run("set_autostart", { enabled: false }),
  },
];

function render() {
  if (!settings) return;
  applyTheme(settings.theme);
  rowsList.replaceChildren(
    ...rows.map((row, i) => {
      const li = document.createElement("li");
      li.className = "row";
      li.classList.toggle("selected", i === selected);
      li.classList.toggle("recording", i === selected && recording !== null);

      const cursor = document.createElement("span");
      cursor.textContent = i === selected ? "❯" : "";
      const label = document.createElement("span");
      label.textContent = row.label;
      const value = document.createElement("span");
      value.className = "value";
      value.textContent = row.value(settings!);
      value.title = value.textContent;

      li.append(cursor, label, value);
      li.addEventListener("click", (e) => {
        selected = i;
        // Clicking the value acts on it, like Enter.
        if (e.target === value) activate(row);
        else render();
      });
      return li;
    }),
  );
  help.textContent = recording !== null ? RECORDING_HELP : HELP;
}

async function run(command: string, args?: Record<string, unknown>) {
  error.hidden = true;
  try {
    settings = await invoke<Settings>(command, args);
  } catch (err) {
    error.textContent = String(err);
    error.hidden = false;
  }
  render();
  // An error message changes the height.
  await fitToContent();
}

function activate(row: Row) {
  if (!settings) return;
  if (row.edit) row.edit();
  else row.cycle?.(settings, 1);
}

function stopRecording() {
  recording = null;
  render();
}

function modifiers(e: KeyboardEvent): string[] {
  const mods = [];
  if (e.ctrlKey) mods.push("Ctrl");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push("Super");
  return mods;
}

/** Hotkey recording: the next key or combination becomes the hotkey; Esc cancels. */
function recordKey(e: KeyboardEvent) {
  const mods = modifiers(e);
  if (e.code === "Escape" && mods.length === 0) {
    stopRecording();
  } else if (MODIFIER_CODES.has(e.code)) {
    recording = [...mods.map((m) => (m === "Super" ? "Win" : m)), "…"].join("+");
    render();
  } else {
    recording = null;
    void run("set_hotkey", { hotkey: [...mods, e.code].join("+") });
  }
}

document.addEventListener("keydown", (e) => {
  if (!settings) return;
  e.preventDefault();
  if (recording !== null) {
    recordKey(e);
    return;
  }
  // Ctrl+C closes like in a terminal (by physical key, so any keyboard layout works).
  if (e.ctrlKey && e.code === "KeyC") {
    void appWindow.close();
    return;
  }
  const row = rows[selected];
  switch (e.key) {
    case "ArrowUp":
      selected = (selected + rows.length - 1) % rows.length;
      render();
      break;
    case "ArrowDown":
      selected = (selected + 1) % rows.length;
      render();
      break;
    case "ArrowLeft":
    case "ArrowRight":
      row.cycle?.(settings, e.key === "ArrowLeft" ? -1 : 1);
      break;
    case "Enter":
    case " ":
      activate(row);
      break;
    case "Delete":
    case "Backspace":
      row.reset?.();
      break;
    case "Escape":
      void appWindow.close();
      break;
  }
});

// Clicking anywhere or leaving the window cancels hotkey recording.
document.addEventListener("mousedown", () => recording !== null && stopRecording(), true);
window.addEventListener("blur", () => recording !== null && stopRecording());

// The window is created hidden: size it to the content first, then show it.
void run("get_settings").then(async () => {
  await reveal();
});
