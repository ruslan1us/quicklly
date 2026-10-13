import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { dateNodes, followDateFormat, toDisplay } from "./dates";
import { markdownNodes } from "./images";
import { linkNodes, shortLink } from "./links";
import { appWindow, fitToContent } from "./popup";
import { followTheme } from "./theme";

/** A note as the reader sees it, so it can be marked done the same way. */
interface Note {
  line: number;
  time: string;
  done: boolean;
  text: string;
}

/** A note with a date of today or earlier that is not done yet. */
interface Due {
  file: string;
  note: Note;
  /** The note's date, `YYYY-MM-DD`. */
  date: string;
  /** How many days ago the date was; 0 when it is today. */
  overdue: number;
}

const title = document.querySelector<HTMLElement>("#title")!;
const rowsList = document.querySelector<HTMLUListElement>("#rows")!;
const error = document.querySelector<HTMLParagraphElement>("#error")!;

/** Beyond this the list scrolls instead of the window growing. */
const MAX_HEIGHT = 400;

let due: Due[] = [];
let selected = 0;

function span(text: string, className = "") {
  const el = document.createElement("span");
  el.textContent = text;
  el.className = className;
  return el;
}

/**
 * The first line of the note's text, as the reader shows it: links short and underlined,
 * images as `[Image #N]` labels and dates in the date format.
 */
function noteText(text: string) {
  const cell = span("");
  const other = (part: string) => dateNodes(part, (rest) => linkNodes(rest, shortLink));
  cell.append(...markdownNodes(toDisplay(text.split("\n")[0]), other));
  return cell;
}

function dueInfo(overdue: number) {
  if (overdue === 0) return span("today");
  return span(`${overdue} ${overdue === 1 ? "day" : "days"} overdue`, "overdue");
}

function render() {
  title.textContent = `Quicklly · Reminders · ${due.length} due`;
  rowsList.replaceChildren(
    ...due.map((item, i) => {
      const li = document.createElement("li");
      li.className = "row due";
      li.classList.toggle("selected", i === selected);
      li.append(
        span(i === selected ? "❯" : ""),
        noteText(item.note.text),
        dueInfo(item.overdue),
      );
      li.addEventListener("click", () => {
        selected = i;
        render();
      });
      return li;
    }),
  );
  rowsList.children[selected]?.scrollIntoView({ block: "nearest" });
}

/** Fits the window to its content, keeping it in its corner. */
async function fit() {
  await fitToContent(MAX_HEIGHT);
  await invoke("place_reminders");
}

function showError(err: unknown) {
  error.textContent = String(err);
  error.hidden = false;
}

/**
 * Reloads the due notes and redraws them; returns false when nothing is due any more, and the
 * window should close. If loading fails, the error is shown instead.
 */
async function load(keepError = false) {
  if (!keepError) error.hidden = true;
  try {
    due = await invoke<Due[]>("list_due");
  } catch (err) {
    due = [];
    showError(err);
    render();
    return true;
  }
  selected = Math.min(selected, Math.max(due.length - 1, 0));
  render();
  return due.length > 0;
}

/** Reloads the list, and closes the window once nothing is due. */
async function refresh(keepError = false) {
  if (await load(keepError)) await fit();
  else await appWindow.close();
}

/** Marks the selected note done, which takes it off the list. */
async function markDone() {
  const item = due[selected];
  if (!item) return;
  let failed = false;
  try {
    await invoke("set_note_done", { name: item.file, note: item.note, done: true });
  } catch (err) {
    showError(err);
    failed = true;
  }
  await refresh(failed);
}

/** Opens the selected note in the reader, like a search result, and closes the reminders. */
async function openNote() {
  const item = due[selected];
  if (!item) return;
  // Hidden first, so the two windows never show at once.
  await appWindow.hide();
  await invoke("open_note", { file: item.file, line: item.note.line });
  await appWindow.close();
}

document.addEventListener("keydown", (e) => {
  // Esc or Ctrl+C (by physical key, so any keyboard layout works) closes like in a terminal.
  if (e.key === "Escape" || (e.ctrlKey && e.code === "KeyC")) {
    e.preventDefault();
    void appWindow.close();
  } else if ((e.key === "ArrowUp" || e.key === "ArrowDown") && due.length > 0) {
    e.preventDefault();
    selected = (selected + (e.key === "ArrowUp" ? -1 : 1) + due.length) % due.length;
    render();
  } else if (e.key === " ") {
    e.preventDefault();
    void markDone();
  } else if (e.key === "Enter") {
    e.preventDefault();
    void openNote();
  }
});

// Notes may be done, changed or added anywhere while the reminders are open.
void listen("notes-changed", () => void refresh());

/** Shows the window once it is filled in and fitted, unless nothing is due after all. */
async function start() {
  if (!(await load())) {
    await appWindow.close();
    return;
  }
  await fitToContent(MAX_HEIGHT);
  await invoke("reveal_reminders");
}

// Shown only once themed and filled in, so it never flickers.
// The zoom has to be known before the window is fitted to its content.
void Promise.all([followTheme(() => void fit()), followDateFormat(render)]).then(start);
