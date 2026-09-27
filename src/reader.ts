import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { blockCaret } from "./blockCaret";
import { appWindow, fitToContent, reveal } from "./popup";
import { followTheme } from "./theme";

interface NoteFile {
  name: string;
  notes: number;
  done: number;
}

type Item =
  | { kind: "day"; date: string }
  | { kind: "note"; line: number; time: string; done: boolean; text: string };

const header = document.querySelector<HTMLElement>("header")!;
const rowsList = document.querySelector<HTMLUListElement>("#rows")!;
const empty = document.querySelector<HTMLParagraphElement>("#empty")!;
const error = document.querySelector<HTMLParagraphElement>("#error")!;
const help = document.querySelector<HTMLElement>("#help")!;

/** Beyond this the list scrolls instead of the window growing. */
const MAX_HEIGHT = 520;

/** The file list, or the notes of `openFile`. */
let view: "files" | "notes" = "files";
let files: NoteFile[] = [];
let selectedFile = 0;
let openFile = "";
let items: Item[] = [];
/** Index into `items` of the selected note (day headings are skipped). */
let selectedItem = -1;
/** Set after the first Del: a second Del deletes the selected note, anything else cancels. */
let confirmingDelete = false;
/** While true, the selected note is shown in `editor` for editing. */
let editing = false;

/** Inline editor for a note, with the same block caret as the note input. */
const editorBox = document.createElement("div");
editorBox.className = "editor";
const editor = document.createElement("textarea");
editor.rows = 1;
editor.spellcheck = false;
const editorCaret = document.createElement("div");
editorCaret.className = "caret";
const editorMirror = document.createElement("div");
editorMirror.className = "mirror";
editorMirror.setAttribute("aria-hidden", "true");
editorBox.append(editor, editorCaret, editorMirror);
const updateEditorCaret = blockCaret(editor, editorCaret, editorMirror);

const dateOf = (name: string) => name.replace(/\.md$/, "");
const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;
/** "12 notes · 3 done", or just "12 notes" when none are done. */
const counts = (notes: number, done: number) =>
  plural(notes, "note") + (done > 0 ? ` · ${done} done` : "");
const noteIndexes = () => items.flatMap((item, i) => (item.kind === "note" ? [i] : []));

/** Today's daily file name, in local time like the notes themselves. */
function todayFile() {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}.md`;
}

function span(text: string, className = "") {
  const el = document.createElement("span");
  el.textContent = text;
  el.className = className;
  return el;
}

/** A list row; plain strings become cells, elements are used as they are. */
function row(
  className: string,
  cells: (string | HTMLElement)[],
  selected: boolean,
  onClick?: () => void,
) {
  const li = document.createElement("li");
  li.className = className;
  li.classList.toggle("selected", selected);
  li.append(...cells.map((cell) => (typeof cell === "string" ? span(cell) : cell)));
  if (onClick) li.addEventListener("click", onClick);
  return li;
}

const BAR_CELLS = 8;

/**
 * Progress bar like ▰▰▰▱▱▱▱▱. It is only full once every note is done, and shows at least one
 * cell as soon as one is.
 */
function progressBar(notes: number, done: number) {
  let filled = Math.round((done / notes) * BAR_CELLS);
  if (done > 0) filled = Math.max(filled, 1);
  if (done < notes) filled = Math.min(filled, BAR_CELLS - 1);
  const bar = span("", "bar");
  bar.append(span("▰".repeat(filled), "filled"), span("▱".repeat(BAR_CELLS - filled)));
  return bar;
}

function renderFiles() {
  const today = todayFile();
  header.textContent = "Quicklly · Notes";
  help.textContent = "↑↓ select · Enter open · → new note · Esc close";
  empty.textContent = "No notes yet.";
  empty.hidden = files.length > 0;
  rowsList.replaceChildren(
    ...files.map((file, i) => {
      const complete = file.notes > 0 && file.done === file.notes;
      const li = row(
        "row file",
        [
          i === selectedFile ? "❯" : "",
          span(file.name === "inbox.md" ? file.name : dateOf(file.name), "name"),
          file.name === today ? "today" : "",
          file.notes > 0 ? progressBar(file.notes, file.done) : "",
          span(
            complete ? "✓ all done" : file.notes > 0 ? `${file.done}/${file.notes}` : "empty",
            "count",
          ),
        ],
        i === selectedFile,
        () => {
          selectedFile = i;
          render();
        },
      );
      li.classList.toggle("complete", complete);
      return li;
    }),
  );
  rowsList.children[selectedFile]?.scrollIntoView({ block: "nearest" });
}

function renderNotes() {
  const count = noteIndexes().length;
  const done = items.filter((item) => item.kind === "note" && item.done).length;
  const title = openFile === "inbox.md" ? openFile : dateOf(openFile);
  header.textContent = `Quicklly · ${title} · ${counts(count, done)}`;
  help.textContent = editing
    ? "Enter save · Shift+Enter new line · Esc cancel"
    : confirmingDelete
      ? "Del again to delete · any other key cancels"
      : "↑↓ select · Space done · Enter edit · Del delete · ← back · Esc close";
  empty.textContent = "No notes in this file.";
  empty.hidden = count > 0;
  rowsList.replaceChildren(
    ...items.map((item, i) => {
      if (item.kind === "day") return row("row day", ["", `── ${item.date}`], false);
      const text = editing && i === selectedItem ? editorBox : item.text;
      const li = row(
        "row note",
        [i === selectedItem ? "❯" : "", item.time, item.done ? "✓" : "", text],
        i === selectedItem,
        () => {
          if (editing) return;
          selectedItem = i;
          render();
        },
      );
      li.classList.toggle("done", item.done && !(editing && i === selectedItem));
      li.classList.toggle("deleting", confirmingDelete && i === selectedItem);
      return li;
    }),
  );
  rowsList.children[selectedItem]?.scrollIntoView({ block: "nearest" });
}

function render() {
  if (view === "files") renderFiles();
  else renderNotes();
}

function showError(err: unknown) {
  error.textContent = String(err);
  error.hidden = false;
}

async function loadFiles() {
  try {
    files = await invoke<NoteFile[]>("list_note_files");
  } catch (err) {
    files = [];
    showError(err);
  }
  selectedFile = Math.min(selectedFile, Math.max(files.length - 1, 0));
}

async function loadNotes() {
  try {
    items = await invoke<Item[]>("read_note_file", { name: openFile });
  } catch (err) {
    items = [];
    showError(err);
  }
  // Keep the selection on reload; a freshly opened file starts at its latest note.
  const notes = noteIndexes();
  if (!notes.includes(selectedItem)) selectedItem = notes[notes.length - 1] ?? -1;
}

/** Reloads the current view from disk and redraws it. */
async function refresh(keepError = false) {
  if (!keepError) error.hidden = true;
  if (view === "files") await loadFiles();
  else await loadNotes();
  render();
  await fitToContent(MAX_HEIGHT);
}

function openSelectedFile() {
  const file = files[selectedFile];
  if (!file) return;
  view = "notes";
  openFile = file.name;
  selectedItem = -1;
  void refresh();
}

/** Goes back to the file list, with the file that was open selected. */
async function backToFiles() {
  view = "files";
  await refresh();
  const i = files.findIndex((file) => file.name === openFile);
  if (i >= 0) {
    selectedFile = i;
    render();
  }
}

/** Opens the note the input window asked for (a search result), if there is one. */
async function openTarget() {
  const target = await invoke<{ file: string; line: number } | null>("take_reader_target");
  if (!target) return;
  view = "notes";
  openFile = target.file;
  selectedItem = -1;
  editing = false;
  confirmingDelete = false;
  await refresh();
  const i = items.findIndex((item) => item.kind === "note" && item.line === target.line);
  if (i >= 0) {
    selectedItem = i;
    render();
  }
}

// A search result opened while the reader is already open.
void listen("reader-target", () => void openTarget());

/**
 * Runs a change to the selected note, then reloads the file. If the change fails (for example
 * because the file changed outside the reader), the error stays visible over the reloaded notes.
 */
async function changeNote(command: string, args: Record<string, unknown> = {}) {
  const note = items[selectedItem];
  if (note?.kind !== "note") return;
  let failed = false;
  try {
    await invoke(command, { name: openFile, note, ...args });
  } catch (err) {
    showError(err);
    failed = true;
  }
  await refresh(failed);
}

/** Deletes the selected note and selects the one that took its place (or the new last one). */
async function deleteNote() {
  const position = noteIndexes().indexOf(selectedItem);
  await changeNote("delete_note");
  const notes = noteIndexes();
  if (notes.length > 0) {
    selectedItem = notes[Math.min(position, notes.length - 1)];
    render();
  }
}

/** Grows the editor with its text, and the window with it. */
function fitEditor() {
  editor.style.height = "0";
  editor.style.height = `${editor.scrollHeight}px`;
  updateEditorCaret();
  void fitToContent(MAX_HEIGHT);
}

function startEdit() {
  const note = items[selectedItem];
  if (note?.kind !== "note") return;
  editing = true;
  editor.value = note.text;
  render();
  editor.focus();
  editor.setSelectionRange(editor.value.length, editor.value.length);
  fitEditor();
}

function cancelEdit() {
  editing = false;
  render();
  void fitToContent(MAX_HEIGHT);
}

/** Saves the edited text; a blank or unchanged text just ends the edit (Del deletes a note). */
async function saveEdit() {
  const note = items[selectedItem];
  const text = editor.value;
  if (note?.kind !== "note" || text.trim() === "" || text === note.text) {
    cancelEdit();
    return;
  }
  editing = false;
  await changeNote("edit_note", { text });
}

editor.addEventListener("input", fitEditor);

function cancelDelete() {
  confirmingDelete = false;
  render();
}

/** Moves an index by `step` through `count` entries, wrapping around. */
const wrap = (index: number, step: number, count: number) => (index + step + count) % count;

function moveNote(step: number) {
  const notes = noteIndexes();
  if (notes.length === 0) return;
  selectedItem = notes[wrap(notes.indexOf(selectedItem), step, notes.length)];
  render();
}

document.addEventListener("keydown", (e) => {
  if (editing) {
    if (e.isComposing) return;
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void saveEdit();
    } else if (
      e.key === "Escape" ||
      (e.ctrlKey && e.code === "KeyC" && editor.selectionStart === editor.selectionEnd)
    ) {
      e.preventDefault();
      cancelEdit();
    }
    // Anything else types into the editor.
    return;
  }
  e.preventDefault();
  // Esc or Ctrl+C (by physical key, so any keyboard layout works) closes like in a terminal.
  if (e.key === "Escape" || (e.ctrlKey && e.code === "KeyC")) {
    void appWindow.close();
    return;
  }
  if (confirmingDelete) {
    confirmingDelete = false;
    if (e.key === "Delete") void deleteNote();
    else render();
    return;
  }
  if (view === "files") {
    if (e.key === "ArrowRight") {
      // → goes back to the note input, the way ← came here.
      void invoke("open_input").then(() => appWindow.close());
    } else if (e.key === "Enter") {
      openSelectedFile();
    } else if (files.length > 0 && (e.key === "ArrowUp" || e.key === "ArrowDown")) {
      selectedFile = wrap(selectedFile, e.key === "ArrowUp" ? -1 : 1, files.length);
      render();
    }
  } else if (e.key === "ArrowLeft") {
    void backToFiles();
  } else if (e.key === "ArrowUp" || e.key === "ArrowDown") {
    moveNote(e.key === "ArrowUp" ? -1 : 1);
  } else if (e.key === " ") {
    const note = items[selectedItem];
    if (note?.kind === "note") void changeNote("set_note_done", { done: !note.done });
  } else if (e.key === "Enter") {
    startEdit();
  } else if (e.key === "Delete" && items[selectedItem]?.kind === "note") {
    confirmingDelete = true;
    render();
  }
});

// A click also cancels a pending delete.
document.addEventListener("mousedown", () => confirmingDelete && cancelDelete(), true);

// Double-click opens a file, like Enter.
rowsList.addEventListener("dblclick", () => view === "files" && openSelectedFile());

// Notes may have changed since the reader was last shown (but don't lose an edit in progress).
window.addEventListener("focus", () => !editing && void refresh());

// Shown only once themed and filled in, so it never flickers.
void Promise.all([followTheme(), refresh()])
  .then(openTarget)
  .then(reveal);
