import { invoke } from "@tauri-apps/api/core";
import { appWindow, fitToContent, reveal } from "./popup";
import { followTheme } from "./theme";

interface NoteFile {
  name: string;
  notes: number;
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

const dateOf = (name: string) => name.replace(/\.md$/, "");
const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;
const noteIndexes = () => items.flatMap((item, i) => (item.kind === "note" ? [i] : []));

/** Today's daily file name, in local time like the notes themselves. */
function todayFile() {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}.md`;
}

function row(className: string, cells: string[], selected: boolean, onClick?: () => void) {
  const li = document.createElement("li");
  li.className = className;
  li.classList.toggle("selected", selected);
  li.append(
    ...cells.map((text) => {
      const span = document.createElement("span");
      span.textContent = text;
      return span;
    }),
  );
  if (onClick) li.addEventListener("click", onClick);
  return li;
}

function renderFiles() {
  const today = todayFile();
  header.textContent = "Quicklly · Notes";
  help.textContent = "↑↓ select · Enter open · → new note · Esc close";
  empty.textContent = "No notes yet.";
  empty.hidden = files.length > 0;
  rowsList.replaceChildren(
    ...files.map((file, i) =>
      row(
        "row file",
        [
          i === selectedFile ? "❯" : "",
          file.name === "inbox.md" ? file.name : dateOf(file.name),
          file.name === today ? "today" : "",
          plural(file.notes, "note"),
        ],
        i === selectedFile,
        () => {
          selectedFile = i;
          render();
        },
      ),
    ),
  );
  rowsList.children[selectedFile]?.scrollIntoView({ block: "nearest" });
}

function renderNotes() {
  const count = noteIndexes().length;
  const title = openFile === "inbox.md" ? openFile : dateOf(openFile);
  header.textContent = `Quicklly · ${title} · ${plural(count, "note")}`;
  help.textContent = "↑↓ select · ← back · Esc close";
  empty.textContent = "No notes in this file.";
  empty.hidden = count > 0;
  rowsList.replaceChildren(
    ...items.map((item, i) => {
      if (item.kind === "day") return row("row day", ["", `── ${item.date}`], false);
      const li = row(
        "row note",
        [i === selectedItem ? "❯" : "", item.time, item.done ? "✓" : "", item.text],
        i === selectedItem,
        () => {
          selectedItem = i;
          render();
        },
      );
      li.classList.toggle("done", item.done);
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
async function refresh() {
  error.hidden = true;
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

function backToFiles() {
  view = "files";
  void refresh();
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
  e.preventDefault();
  // Esc or Ctrl+C (by physical key, so any keyboard layout works) closes like in a terminal.
  if (e.key === "Escape" || (e.ctrlKey && e.code === "KeyC")) {
    void appWindow.close();
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
    backToFiles();
  } else if (e.key === "ArrowUp" || e.key === "ArrowDown") {
    moveNote(e.key === "ArrowUp" ? -1 : 1);
  }
});

// Double-click opens a file, like Enter.
rowsList.addEventListener("dblclick", () => view === "files" && openSelectedFile());

// Notes may have changed since the reader was last shown.
window.addEventListener("focus", () => void refresh());

// Shown only once themed and filled in, so it never flickers.
void Promise.all([followTheme(), refresh()]).then(reveal);
