import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { blockCaret } from "./blockCaret";
import { followTheme } from "./theme";

const appWindow = getCurrentWindow();
const text = document.querySelector<HTMLTextAreaElement>("#text")!;
const numbers = document.querySelector<HTMLDivElement>("#numbers")!;
const backdrop = document.querySelector<HTMLDivElement>("#backdrop")!;
const status = document.querySelector<HTMLSpanElement>("#status")!;
const pin = document.querySelector<HTMLSpanElement>("#pin")!;
const help = document.querySelector<HTMLSpanElement>("#help")!;
const title = document.querySelector<HTMLSpanElement>("#title")!;

/** A note from the reader (Shift+Enter there), edited here in place of the draft. */
interface PadEdit {
  name: string;
  note: { line: number; time: string; done: boolean; text: string };
}
let editing: PadEdit | null = null;

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

/** A `#tag`, as the notes files understand it: `#` at the start of a word, then a letter. */
const TAG = /(^|\s)(#\p{L}[\p{L}\p{N}_-]*)/gu;
/** A list item: indentation, then `- [ ] `, `- [x] `, `- `, `* ` or `1. `. */
const LIST_ITEM = /^(\s*)(- \[[ xX]\] |- |\* |(\d+)\. )/;

/** Number of the line the cursor is on, counting from 1. */
const currentLine = () => text.value.slice(0, text.selectionStart).split("\n").length;

function span(content: string, className: string) {
  const el = document.createElement("span");
  el.textContent = content;
  el.className = className;
  return el;
}

/** Appends `content` to `parent`, with its `#tags` marked. */
function appendWithTags(parent: Node, content: string) {
  let from = 0;
  for (const match of content.matchAll(TAG)) {
    const at = match.index + match[1].length;
    parent.appendChild(document.createTextNode(content.slice(from, at)));
    parent.appendChild(span(match[2], "tag"));
    from = at + match[2].length;
  }
  parent.appendChild(document.createTextNode(content.slice(from)));
}

/** One line of text as it is shown under the (transparent) text: list markers and tags marked. */
function renderLine(line: string) {
  const div = document.createElement("div");
  div.className = "line";
  if (line === "") {
    div.textContent = ZERO_WIDTH_SPACE;
    return div;
  }
  const item = LIST_ITEM.exec(line);
  if (!item) {
    appendWithTags(div, line);
    return div;
  }
  const marker = item[2];
  const checkbox = marker.startsWith("- [");
  div.append(item[1], span(marker, checkbox ? "check" : "bullet"));
  const rest = line.slice(item[0].length);
  if (checkbox && marker !== "- [ ] ") {
    // A checked item: its text is done.
    const done = span("", "done");
    appendWithTags(done, rest);
    div.append(done);
  } else {
    appendWithTags(div, rest);
  }
  return div;
}

/**
 * Draws the text with its highlighting behind the transparent text area, and one line number
 * per line, as tall as the rows that line wraps to.
 */
function renderLines() {
  backdrop.replaceChildren(...text.value.split("\n").map(renderLine));
  numbers.replaceChildren(
    ...[...backdrop.children].map((line, i) => {
      const div = document.createElement("div");
      div.textContent = String(i + 1);
      div.style.height = `${(line as HTMLElement).offsetHeight}px`;
      return div;
    }),
  );
  markCurrentLine();
  syncScroll();
}

function markCurrentLine() {
  const current = currentLine() - 1;
  for (const list of [numbers.children, backdrop.children]) {
    [...list].forEach((el, i) => el.classList.toggle("current", i === current));
  }
}

function syncScroll() {
  const offset = `translateY(${-text.scrollTop}px)`;
  numbers.style.transform = offset;
  backdrop.style.transform = offset;
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

function renderHelp() {
  title.textContent = editing ? `Quicklly · Pad · editing ${editing.note.time}` : "Quicklly · Pad";
  if (editing) {
    help.textContent = "Ctrl+Enter save · Esc cancel";
    return;
  }
  const first = text.value === "" ? "← quick line" : "Ctrl+Enter save";
  help.textContent = `${first} · Ctrl+P pin · Esc close`;
}

function refresh() {
  renderLines();
  renderStatus();
  renderHelp();
  updateCaret();
}

/** Types `content` over the selection, keeping Ctrl+Z working. */
function insert(content: string) {
  document.execCommand("insertText", false, content);
}

/**
 * Enter in a list item starts the next one (`- `, `- [ ] `, the next number); Enter in an
 * empty item ends the list. Returns false when the line is not a list item.
 */
function continueList() {
  const { selectionStart: start, selectionEnd: end, value } = text;
  if (start !== end) return false;
  const lineStart = value.lastIndexOf("\n", start - 1) + 1;
  const item = LIST_ITEM.exec(value.slice(lineStart, start));
  if (!item) return false;
  const found = value.indexOf("\n", start);
  const lineEnd = found < 0 ? value.length : found;
  if (value.slice(lineStart + item[0].length, lineEnd).trim() === "") {
    text.setSelectionRange(lineStart, lineEnd);
    insert("");
    return true;
  }
  const [, indentation, marker, number] = item;
  const next = number
    ? `${Number(number) + 1}. `
    : marker.startsWith("- [")
      ? "- [ ] "
      : marker;
  insert(`\n${indentation}${next}`);
  return true;
}

/** Tab indents by two spaces (every selected line, if several); Shift+Tab takes them away. */
function indent(outdent: boolean) {
  const { selectionStart: start, selectionEnd: end, value } = text;
  const multiLine = value.slice(start, end).includes("\n");
  if (!outdent && !multiLine) {
    insert("  ");
    return;
  }
  const blockStart = value.lastIndexOf("\n", start - 1) + 1;
  const found = value.indexOf("\n", end);
  const blockEnd = found < 0 ? value.length : found;
  const changed = value
    .slice(blockStart, blockEnd)
    .split("\n")
    .map((line) => (outdent ? line.replace(/^ {1,2}/, "") : `  ${line}`))
    .join("\n");
  text.setSelectionRange(blockStart, blockEnd);
  insert(changed);
  if (multiLine) text.setSelectionRange(blockStart, blockStart + changed.length);
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
  // Leaving an edit cancels it; the draft is kept either way.
  if (editing) await stopEditing();
  await saveDraft();
  await appWindow.hide();
}

/** Puts the draft aside to edit the note the reader asked for, if there is one. */
async function startEditing() {
  const edit = await invoke<PadEdit | null>("take_pad_edit");
  if (!edit) return;
  if (!editing) await saveDraft();
  editing = edit;
  text.value = edit.note.text;
  text.setSelectionRange(text.value.length, text.value.length);
  refresh();
}

/** Ends an edit and brings the draft back. */
async function stopEditing() {
  editing = null;
  text.value = await invoke<string>("get_pad_draft");
  text.setSelectionRange(text.value.length, text.value.length);
  refresh();
}

/** Saves the edited note in its file, where the reader shows it. */
async function saveEdit(edit: PadEdit) {
  if (!text.value.trim()) return;
  try {
    await invoke("edit_note", { name: edit.name, note: edit.note, text: text.value });
  } catch (err) {
    showError(err);
    return;
  }
  await stopEditing();
  if (pinned) flashSaved();
  else await hide();
}

/**
 * Saves the text as a note, like the input window does, and starts over with an empty Pad,
 * which then hides unless it is pinned.
 */
async function saveNote() {
  if (editing) {
    await saveEdit(editing);
    return;
  }
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
  const plain = !e.shiftKey && !e.ctrlKey && !e.altKey && !e.metaKey;
  if (e.key === "Enter" && e.ctrlKey) {
    e.preventDefault();
    void saveNote();
  } else if (e.key === "Enter" && plain) {
    if (continueList()) e.preventDefault();
  } else if (e.key === "Tab" && !e.ctrlKey && !e.altKey) {
    e.preventDefault();
    indent(e.shiftKey);
  } else if (e.key === "ArrowLeft" && text.value === "" && plain && !editing) {
    // ← in an empty Pad goes back to the input window, the way → came here.
    e.preventDefault();
    void invoke("open_input").then(() => (pinned ? undefined : hide()));
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
  if (!editing) saveDraftSoon();
});
text.addEventListener("scroll", () => {
  syncScroll();
  updateCaret();
});
document.addEventListener("selectionchange", () => {
  renderStatus();
  markCurrentLine();
});

// A note to edit, sent while the Pad is already open.
void listen("pad-edit", () => void startEditing());

// Text moved here from the input window with Ctrl+E (it shows once an edit is over).
void listen("pad-draft-changed", async () => {
  if (editing) return;
  text.value = await invoke<string>("get_pad_draft");
  text.setSelectionRange(text.value.length, text.value.length);
  refresh();
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
  await startEditing();
  await appWindow.show();
  await appWindow.setFocus();
  text.focus();
  refresh();
});
