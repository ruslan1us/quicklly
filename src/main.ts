import { invoke } from "@tauri-apps/api/core";
import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";
import { followTheme } from "./theme";

const input = document.querySelector<HTMLTextAreaElement>("#note")!;
const appWindow = getCurrentWindow();

const measure = document.querySelector<HTMLDivElement>("#measure")!;
const caret = document.querySelector<HTMLDivElement>("#caret")!;
const caretMeasure = document.querySelector<HTMLDivElement>("#caret-measure")!;

/** The window grows with the note up to 10 lines, then the note scrolls. */
const LINE_HEIGHT = 26;
const MAX_HEIGHT = 56 + LINE_HEIGHT * 9;
let windowHeight = 0;
let scrollable = false;
const ZERO_WIDTH_SPACE = String.fromCharCode(0x200b);
const NBSP = String.fromCharCode(0xa0);

/**
 * Resizes the window to fit the text; the note fills the window and follows it.
 * The height is measured on an invisible copy, so the note itself never jumps.
 */
async function fit() {
  // A trailing new line only counts as a line if something follows it.
  measure.textContent = input.value + ZERO_WIDTH_SPACE;
  const needed = measure.offsetHeight;
  const height = Math.min(needed, MAX_HEIGHT);
  scrollable = needed > MAX_HEIGHT;
  input.style.overflowY = scrollable ? "auto" : "hidden";
  if (!scrollable) input.scrollTop = 0;
  if (height !== windowHeight) {
    windowHeight = height;
    await appWindow.setSize(new LogicalSize(window.innerWidth, height));
  }
}

// Until the window has grown, a new line would scroll the note to the caret and back.
input.addEventListener("scroll", () => {
  if (!scrollable) input.scrollTop = 0;
  updateCaret();
});

/**
 * Draws the terminal-style block caret over the character at the cursor, found by laying
 * out the text before the cursor on an invisible copy of the note.
 */
function updateCaret() {
  const { selectionStart, selectionEnd, value } = input;
  // A selection is shown by its highlight instead.
  if (document.activeElement !== input || selectionStart !== selectionEnd) {
    caret.hidden = true;
    return;
  }
  // Over an empty note the caret covers the first letter of the placeholder, like a terminal.
  const under = value === "" ? input.placeholder[0] : value[selectionStart];
  const marker = document.createElement("span");
  marker.textContent = under && under !== "\n" ? under : NBSP;
  caretMeasure.replaceChildren(value.slice(0, selectionStart), marker);

  const rect = marker.getBoundingClientRect();
  caret.textContent = marker.textContent;
  caret.style.left = `${rect.left}px`;
  // The marker box is the glyph height; centre the caret on the 26px line instead.
  const lineTop = rect.top - (LINE_HEIGHT - rect.height) / 2;
  caret.style.top = `${lineTop - input.scrollTop}px`;
  caret.style.width = `${rect.width}px`;
  caret.hidden = false;
}

document.addEventListener("selectionchange", updateCaret);
input.addEventListener("focus", updateCaret);
input.addEventListener("blur", updateCaret);

/** Notes saved while the app runs, oldest first; browsed with ↑/↓ like a shell history. */
const history: string[] = [];
const HISTORY_LIMIT = 100;
/** Position while browsing; `history.length` stands for the empty field. */
let historyIndex = 0;

/** True while the field shows an unedited note from the history. */
function browsingHistory() {
  return historyIndex < history.length && input.value === history[historyIndex];
}

function showHistory(index: number) {
  historyIndex = index;
  input.value = history[index] ?? "";
  input.setSelectionRange(input.value.length, input.value.length);
  void fit();
}

function remember(text: string) {
  if (history[history.length - 1] !== text) history.push(text);
  if (history.length > HISTORY_LIMIT) history.shift();
}

function clearError() {
  input.classList.remove("error");
  input.title = "";
}

async function hide() {
  input.value = "";
  historyIndex = history.length;
  clearError();
  await appWindow.hide();
  await fit();
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
    remember(text);
    await hide();
  } catch (err) {
    input.classList.add("error");
    input.title = String(err);
  }
}

input.addEventListener("keydown", (e) => {
  if (e.isComposing) return;
  // Shift+Enter keeps its default: a new line in the note.
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    void save();
  } else if (
    e.key === "Escape" ||
    // Ctrl+C closes like in a terminal, unless there is a selection to copy.
    (e.ctrlKey && e.code === "KeyC" && input.selectionStart === input.selectionEnd)
  ) {
    e.preventDefault();
    void hide();
  } else if (e.shiftKey || e.ctrlKey || e.altKey || e.metaKey) {
    return;
  } else if (e.key === "ArrowUp" && (input.value === "" || browsingHistory())) {
    // In an empty field or while browsing, ↑/↓ walk the history; otherwise they move the caret.
    e.preventDefault();
    if (historyIndex > 0) showHistory(historyIndex - 1);
  } else if (e.key === "ArrowDown" && browsingHistory()) {
    e.preventDefault();
    showHistory(historyIndex + 1);
  }
});

input.addEventListener("input", () => {
  clearError();
  updateCaret();
  void fit();
});

// The window is hidden rather than destroyed, so refocus the input on every show.
window.addEventListener("focus", () => input.focus());

void followTheme();
