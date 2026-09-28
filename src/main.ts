import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";
import { blockCaret } from "./blockCaret";
import { BASE_WIDTH } from "./popup";
import { followTheme, zoom } from "./theme";

const input = document.querySelector<HTMLTextAreaElement>("#note")!;
const appWindow = getCurrentWindow();

const measure = document.querySelector<HTMLDivElement>("#measure")!;
const caret = document.querySelector<HTMLDivElement>("#caret")!;
const caretMeasure = document.querySelector<HTMLDivElement>("#caret-measure")!;
const results = document.querySelector<HTMLUListElement>("#results")!;
const hint = document.querySelector<HTMLSpanElement>(".hint")!;

/** The window grows with the note up to 10 lines, then the note scrolls. */
const LINE_HEIGHT = 26;
const MAX_HEIGHT = 56 + LINE_HEIGHT * 9;
let windowHeight = 0;
let scrollable = false;
const ZERO_WIDTH_SPACE = String.fromCharCode(0x200b);

/**
 * Resizes the note and the window to fit the text, plus the search results below it.
 * The height is measured on an invisible copy, so the note itself never jumps.
 */
async function fit() {
  // A trailing new line only counts as a line if something follows it.
  measure.textContent = input.value + ZERO_WIDTH_SPACE;
  const needed = measure.offsetHeight;
  const noteHeight = Math.min(needed, MAX_HEIGHT);
  scrollable = needed > MAX_HEIGHT;
  input.style.height = `${noteHeight}px`;
  input.style.overflowY = scrollable ? "auto" : "hidden";
  if (!scrollable) input.scrollTop = 0;
  const height = noteHeight + (results.hidden ? 0 : results.offsetHeight);
  if (height !== windowHeight) {
    windowHeight = height;
    // The window is sized in logical pixels: CSS pixels times the zoom.
    await appWindow.setSize(new LogicalSize(BASE_WIDTH * zoom, height * zoom));
  }
}

// Until the window has grown, a new line would scroll the note to the caret and back.
input.addEventListener("scroll", () => {
  if (!scrollable) input.scrollTop = 0;
  updateCaret();
});

const updateCaret = blockCaret(input, caret, caretMeasure);

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

/** A note found by search. */
interface Hit {
  file: string;
  line: number;
  day: string;
  time: string;
  done: boolean;
  text: string;
}

let hits: Hit[] = [];
let selectedHit = 0;
/** Numbers the searches, so a slow one can't overwrite the results of a newer one. */
let searchCount = 0;

/**
 * Search mode: typing `?` into an empty note switches to it, like `!` in a terminal prompt.
 * The `?` becomes a prompt in front of the text, and Backspace on an empty query leaves it.
 */
let searchMode = false;
const searching = () => searchMode;
const searchQuery = () => input.value.trim();
const NOTE_PLACEHOLDER = input.placeholder;

function setSearchMode(on: boolean) {
  searchMode = on;
  document.body.classList.toggle("search", on);
  input.placeholder = on ? "Search notes…" : NOTE_PLACEHOLDER;
  updateCaret();
}

/** `text` as nodes, with each occurrence of `query` (ignoring case) marked. */
function highlight(text: string, query: string) {
  const parts: Node[] = [];
  const lower = text.toLowerCase();
  const needle = query.toLowerCase();
  let from = 0;
  for (let at = lower.indexOf(needle); needle && at >= 0; at = lower.indexOf(needle, from)) {
    parts.push(document.createTextNode(text.slice(from, at)));
    const mark = document.createElement("mark");
    mark.textContent = text.slice(at, at + needle.length);
    parts.push(mark);
    from = at + needle.length;
  }
  parts.push(document.createTextNode(text.slice(from)));
  return parts;
}

function cell(text: string, className = "") {
  const span = document.createElement("span");
  span.textContent = text;
  span.className = className;
  return span;
}

function renderResults() {
  results.hidden = !searching();
  if (results.hidden) {
    results.replaceChildren();
    return;
  }
  const query = searchQuery();
  if (!query || hits.length === 0) {
    const li = document.createElement("li");
    li.textContent = query ? "No matches" : "Type to search your notes";
    results.replaceChildren(li);
    return;
  }
  results.replaceChildren(
    ...hits.map((hit, i) => {
      const li = document.createElement("li");
      li.className = "hit";
      li.classList.toggle("selected", i === selectedHit);
      li.classList.toggle("done", hit.done);
      // One line per result: extra lines of a note follow a ⏎.
      const text = cell("", "text");
      text.append(...highlight(hit.text.split("\n").join(" ⏎ "), query));
      li.append(
        cell(i === selectedHit ? "❯" : ""),
        cell(hit.day),
        cell(hit.time),
        cell(hit.done ? "✓" : ""),
        text,
      );
      // mousedown, so the note keeps the focus.
      li.addEventListener("mousedown", (e) => {
        e.preventDefault();
        selectedHit = i;
        void openHit();
      });
      return li;
    }),
  );
  results.children[selectedHit]?.scrollIntoView({ block: "nearest" });
}

async function search() {
  const count = ++searchCount;
  let found: Hit[] = [];
  if (searching() && searchQuery()) {
    try {
      found = await invoke<Hit[]>("search_notes", { query: searchQuery() });
    } catch (err) {
      showError(err);
    }
  }
  if (count !== searchCount) return;
  hits = found;
  selectedHit = 0;
  renderResults();
  await fit();
}

/** Opens the selected result in the reader. */
async function openHit() {
  const hit = hits[selectedHit];
  if (!hit) return;
  await invoke("open_note", { file: hit.file, line: hit.line });
  await hide();
}

function moveHit(step: number) {
  if (hits.length === 0) return;
  selectedHit = (selectedHit + step + hits.length) % hits.length;
  renderResults();
}

function showError(err: unknown) {
  input.classList.add("error");
  input.title = String(err);
}

function clearError() {
  input.classList.remove("error");
  input.title = "";
}

async function hide() {
  input.value = "";
  setSearchMode(false);
  historyIndex = history.length;
  hits = [];
  searchCount++;
  clearError();
  renderResults();
  await appWindow.hide();
  await fit();
}

async function save() {
  const text = input.value.trim();
  if (!text) {
    await hide();
    return;
  }
  if (searching()) {
    await openHit();
    return;
  }
  if (text === "/config") {
    await invoke("open_settings");
    await hide();
    return;
  }
  if (text === "/pad") {
    await invoke("open_pad");
    await hide();
    return;
  }
  if (text === "/exit") {
    await invoke("exit_app");
    return;
  }
  if (text === "/update") {
    input.value = "";
    showHint("updating…");
    try {
      // On success the app closes, updates and starts again.
      await invoke("install_update");
    } catch (err) {
      showHint(HINT);
      showError(err);
    }
    return;
  }
  try {
    await invoke("save_note", { text });
    remember(text);
    await hide();
  } catch (err) {
    showError(err);
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
  } else if (e.ctrlKey && e.code === "KeyE" && !searching()) {
    // Ctrl+E moves what was typed into the Pad, to go on writing there.
    e.preventDefault();
    void invoke("expand_to_pad", { text: input.value }).then(hide, showError);
  } else if (e.shiftKey || e.ctrlKey || e.altKey || e.metaKey) {
    return;
  } else if (searching() && e.key === "Backspace" && input.value === "") {
    // Backspace on an empty query leaves search mode.
    e.preventDefault();
    setSearchMode(false);
    void search();
  } else if (searching() && (e.key === "ArrowUp" || e.key === "ArrowDown")) {
    // While searching, ↑/↓ pick a result.
    e.preventDefault();
    moveHit(e.key === "ArrowUp" ? -1 : 1);
  } else if (e.key === "ArrowLeft" && input.value === "") {
    // ← in an empty note opens the notes reader.
    e.preventDefault();
    void invoke("open_reader").then(hide);
  } else if (e.key === "ArrowRight" && input.value === "" && !searching()) {
    // → in an empty note opens the Pad for longer notes.
    e.preventDefault();
    void invoke("open_pad").then(hide);
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
  if (!searchMode && input.value === "?") {
    input.value = "";
    setSearchMode(true);
  }
  clearError();
  updateCaret();
  void search();
});

const HINT = hint.textContent ?? "";
/** Version of a newer release that /update can install. */
let updateVersion: string | null = null;
let hintTimer: number | undefined;

function showHint(text: string, className = "") {
  window.clearTimeout(hintTimer);
  hint.textContent = text;
  hint.className = `hint ${className}`.trim();
}

/** For two seconds after the window opens, the hint announces a waiting update. */
function announceUpdate() {
  if (!updateVersion) return;
  showHint(`update v${updateVersion} available · /update`, "update");
  hintTimer = window.setTimeout(() => showHint(HINT), 2000);
}

// A found release, or null once automatic updates are turned off.
void listen<string | null>("update-available", (event) => (updateVersion = event.payload));
void invoke<string | null>("get_update").then((version) => (updateVersion = version));

// The window is hidden rather than destroyed, so refocus the input on every show.
window.addEventListener("focus", () => {
  input.focus();
  announceUpdate();
});

// The window is fitted to the note once the scale is known, and again whenever it changes.
function refit() {
  windowHeight = 0;
  void fit();
}
void followTheme(refit).then(refit);
