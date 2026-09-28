import { invoke } from "@tauri-apps/api/core";
import { appWindow, fitToContent, reveal } from "./popup";
import { type Theme, applyTheme, followTheme } from "./theme";

interface Settings {
  notesDir: string;
  notesDirIsDefault: boolean;
  mode: "daily" | "inbox";
  hotkey: string;
  hotkeyIsDefault: boolean;
  theme: Theme;
  autostart: boolean;
  autoUpdate: boolean;
  transparency: number;
  scale: number;
  hotkeyTarget: "input" | "pad";
  padPosition: PadPosition;
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

const TRANSPARENCY = [0, 10, 20, 30, 40, 50, 60, 70, 80];
const SCALES = [80, 90, 100, 110, 125, 150];

/** The value `step` places from `current` in `values`, wrapping around. */
function stepThrough(values: number[], current: number, step: number) {
  const at = values.indexOf(current);
  return values[(Math.max(at, 0) + step + values.length) % values.length];
}

type PadPosition = "topRight" | "topLeft" | "bottomRight" | "bottomLeft" | "center";
const PAD_POSITIONS: { position: PadPosition; name: string }[] = [
  { position: "topRight", name: "Top right" },
  { position: "topLeft", name: "Top left" },
  { position: "bottomRight", name: "Bottom right" },
  { position: "bottomLeft", name: "Bottom left" },
  { position: "center", name: "Center" },
];
const padPositionIndex = (s: Settings) =>
  PAD_POSITIONS.findIndex((p) => p.position === s.padPosition);

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
    label: "Hotkey opens",
    value: (s) => choice(s.hotkeyTarget === "pad" ? "Pad" : "Quick line"),
    cycle: (s) =>
      void run("set_hotkey_target", { target: s.hotkeyTarget === "pad" ? "input" : "pad" }),
    reset: () => void run("set_hotkey_target", { target: "input" }),
  },
  {
    label: "Pad position",
    value: (s) => choice(PAD_POSITIONS[padPositionIndex(s)]?.name ?? s.padPosition),
    cycle: (s, step) => {
      const count = PAD_POSITIONS.length;
      const next = PAD_POSITIONS[(padPositionIndex(s) + step + count) % count];
      void run("set_pad_position", { position: next.position });
    },
    reset: () => void run("set_pad_position", { position: "topRight" }),
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
    label: "Transparency",
    value: (s) => choice(s.transparency === 0 ? "Off" : `${s.transparency}%`),
    cycle: (s, step) =>
      void run("set_transparency", { percent: stepThrough(TRANSPARENCY, s.transparency, step) }),
    reset: () => void run("set_transparency", { percent: 30 }),
  },
  {
    label: "Scale",
    value: (s) => choice(`${s.scale}%`),
    cycle: (s, step) => void run("set_scale", { percent: stepThrough(SCALES, s.scale, step) }),
    reset: () => void run("set_scale", { percent: 100 }),
  },
  {
    label: "Start with Windows",
    value: (s) => choice(s.autostart ? "On" : "Off"),
    cycle: (s) => void run("set_autostart", { enabled: !s.autostart }),
    reset: () => void run("set_autostart", { enabled: false }),
  },
  {
    label: "Auto update",
    value: (s) => choice(s.autoUpdate ? "On" : "Off"),
    cycle: (s) => void run("set_auto_update", { enabled: !s.autoUpdate }),
    reset: () => void run("set_auto_update", { enabled: true }),
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
// The zoom has to be known before the window is fitted to its content.
void followTheme(() => void fitToContent())
  .then(() => run("get_settings"))
  .then(reveal);
