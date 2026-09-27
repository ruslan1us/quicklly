import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { blockCaret } from "./blockCaret";
import { followTheme } from "./theme";

const appWindow = getCurrentWindow();
const text = document.querySelector<HTMLTextAreaElement>("#text")!;
const numbers = document.querySelector<HTMLDivElement>("#numbers")!;
const linesMirror = document.querySelector<HTMLDivElement>("#lines-mirror")!;
const status = document.querySelector<HTMLSpanElement>("#status")!;
const pin = document.querySelector<HTMLSpanElement>("#pin")!;

/** A pinned Pad stays open over other windows; an unpinned one hides like the input window. */
let pinned = false;

function renderPin() {
  pin.textContent = pinned ? "● pinned" : "○ pin";
  pin.classList.toggle("pinned", pinned);
}

async function togglePin() {
  try {
    await invoke("set_pad_pinned", { pinned: !pinned });
    pinned = !pinned;
    renderPin();
  } catch (err) {
    showError(err);
  }
}

pin.addEventListener("click", () => void togglePin());

const updateCaret = blockCaret(
  text,
  document.querySelector<HTMLDivElement>("#caret")!,
  document.querySelector<HTMLDivElement>("#caret-mirror")!,
);

const ZERO_WIDTH_SPACE = String.fromCharCode(0x200b);

/** Number of the line the cursor is on, counting from 1. */
const currentLine = () => text.value.slice(0, text.selectionStart).split("\n").length;

/**
 * Draws one line number per line of text, as tall as the rows that line wraps to, which are
 * measured on an invisible copy of the text.
 */
function renderNumbers() {
  const lines = text.value.split("\n");
  linesMirror.replaceChildren(
    ...lines.map((line) => {
      const div = document.createElement("div");
      div.textContent = line || ZERO_WIDTH_SPACE;
      return div;
    }),
  );
  numbers.replaceChildren(
    ...[...linesMirror.children].map((row, i) => {
      const div = document.createElement("div");
      div.textContent = String(i + 1);
      div.style.height = `${(row as HTMLElement).offsetHeight}px`;
      return div;
    }),
  );
  markCurrentLine();
  syncScroll();
}

function markCurrentLine() {
  const current = currentLine();
  [...numbers.children].forEach((div, i) => div.classList.toggle("current", i + 1 === current));
}

function syncScroll() {
  numbers.style.transform = `translateY(${-text.scrollTop}px)`;
}

/** Until when the status says a note was just saved (in a pinned Pad, which stays open). */
let savedUntil = 0;

function flashSaved() {
  savedUntil = Date.now() + 1500;
  renderStatus();
  window.setTimeout(renderStatus, 1500);
}

function renderStatus() {
  if (Date.now() < savedUntil) {
    status.className = "saved";
    status.textContent = "saved ✓";
    return;
  }
  const before = text.value.slice(0, text.selectionStart);
  const column = before.length - before.lastIndexOf("\n");
  const chars = text.value.length;
  status.className = "";
  status.textContent = `Ln ${currentLine()}, Col ${column} · ${chars} ${chars === 1 ? "char" : "chars"}`;
}

function showError(err: unknown) {
  status.className = "error";
  status.textContent = String(err);
}

function refresh() {
  renderNumbers();
  renderStatus();
  updateCaret();
}

/** Saves the draft shortly after typing stops, so it survives closing the Pad or the app. */
let draftTimer: number | undefined;
function saveDraftSoon() {
  window.clearTimeout(draftTimer);
  draftTimer = window.setTimeout(saveDraft, 300);
}

async function saveDraft() {
  window.clearTimeout(draftTimer);
  try {
    await invoke("set_pad_draft", { text: text.value });
  } catch (err) {
    showError(err);
  }
}

async function hide() {
  await saveDraft();
  await appWindow.hide();
}

/**
 * Saves the text as a note, like the input window does, and starts over with an empty Pad,
 * which then hides unless it is pinned.
 */
async function saveNote() {
  if (!text.value.trim()) return;
  try {
    await invoke("save_note", { text: text.value });
  } catch (err) {
    showError(err);
    return;
  }
  text.value = "";
  refresh();
  if (pinned) {
    flashSaved();
    await saveDraft();
  } else {
    await hide();
  }
}

text.addEventListener("keydown", (e) => {
  if (e.isComposing) return;
  if (e.key === "Enter" && e.ctrlKey) {
    e.preventDefault();
    void saveNote();
  } else if (e.ctrlKey && e.code === "KeyP") {
    e.preventDefault();
    void togglePin();
  } else if (
    e.key === "Escape" ||
    // Ctrl+C closes like in a terminal, unless there is a selection to copy.
    (e.ctrlKey && e.code === "KeyC" && text.selectionStart === text.selectionEnd)
  ) {
    e.preventDefault();
    void hide();
  }
});

text.addEventListener("input", () => {
  refresh();
  saveDraftSoon();
});
text.addEventListener("scroll", () => {
  syncScroll();
  updateCaret();
});
document.addEventListener("selectionchange", () => {
  renderStatus();
  markCurrentLine();
});

// Resizing a pinned Pad changes where lines wrap.
window.addEventListener("resize", refresh);

// The window is hidden rather than destroyed, so refocus the text on every show.
window.addEventListener("focus", () => {
  text.focus();
  refresh();
});

// Shown only once themed and filled with the draft, so it never flickers.
void Promise.all([
  followTheme(),
  invoke<string>("get_pad_draft"),
  invoke<boolean>("get_pad_pinned"),
]).then(async ([, draft, isPinned]) => {
  pinned = isPinned;
  renderPin();
  text.value = draft;
  text.setSelectionRange(draft.length, draft.length);
  await appWindow.show();
  await appWindow.setFocus();
  text.focus();
  refresh();
});
