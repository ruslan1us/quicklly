import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { dateNodes, followDateFormat, shownDate, toDisplay } from "../../shared/dates";
import { markdownNodes } from "../../shared/images";
import { linkNodes, shortLink } from "../../shared/links";
import { appWindow, fitToContent } from "../../shared/popup";
import { followTheme } from "../../shared/theme";

/** A note as the reader sees it, so it can be marked done the same way. */
interface Note {
  line: number;
  time: string;
  done: boolean;
  text: string;
}

/** A note with a date of today or earlier (or, ahead, a later one) that is not done yet. */
interface Due {
  file: string;
  note: Note;
  /** The note's date, `YYYY-MM-DD`. */
  date: string;
  /** How many days ago the date was; 0 when it is today, less than 0 when it is ahead. */
  overdue: number;
}

const title = document.querySelector<HTMLElement>("#title")!;
const rowsList = document.querySelector<HTMLUListElement>("#rows")!;
const empty = document.querySelector<HTMLParagraphElement>("#empty")!;
const error = document.querySelector<HTMLParagraphElement>("#error")!;
const help = document.querySelector<HTMLElement>("#help")!;

/** Beyond this the list scrolls instead of the window growing. */
const MAX_HEIGHT = 400;

/** Which notes are shown: those due now (today or earlier), or those ahead. Tab switches. */
let ahead = false;
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
  if (overdue === -1) return span("tomorrow");
  if (overdue < 0) return span(`in ${-overdue} days`);
  return span(`${overdue} ${overdue === 1 ? "day" : "days"} overdue`, "overdue");
}

/** The line above the notes ahead of one day: its date. */
function dayRow(date: string) {
  const li = document.createElement("li");
  li.className = "row day";
  li.append(span(""), span(`── ${shownDate(date)}`));
  return li;
}

function render() {
  title.textContent = `Quicklly · Reminders · ${due.length} ${ahead ? "ahead" : "due"}`;
  // Snoozing is for the notes due: ahead, S and W would bring a note forward instead.
  help.textContent = ahead
    ? "Space done · Enter open · Tab due · Esc close"
    : "Space done · S tomorrow · W week · Enter open · Tab ahead · Esc close";
  empty.textContent = ahead ? "Nothing ahead." : "Nothing due.";
  empty.hidden = due.length > 0;
  const rows: HTMLLIElement[] = [];
  due.forEach((item, i) => {
    // Ahead, the notes of each day come under its date.
    if (ahead && item.date !== due[i - 1]?.date) rows.push(dayRow(item.date));
    const li = document.createElement("li");
    li.className = "row due";
    li.classList.toggle("selected", i === selected);
    li.append(span(i === selected ? "❯" : ""), noteText(item.note.text), dueInfo(item.overdue));
    li.addEventListener("click", () => {
      selected = i;
      render();
    });
    rows.push(li);
  });
  rowsList.replaceChildren(...rows);
  const row = rowsList.querySelector(".selected");
  // The date above the first note of a day shows too.
  const above = row?.previousElementSibling;
  if (above?.classList.contains("day")) above.scrollIntoView({ block: "nearest" });
  row?.scrollIntoView({ block: "nearest" });
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
 * Reloads the notes shown and redraws them; returns false when the last of them is gone, and
 * the window should close. If loading fails, the error is shown instead.
 */
async function load(keepError = false) {
  if (!keepError) error.hidden = true;
  const had = due.length;
  try {
    due = await invoke<Due[]>("list_due", { ahead });
  } catch (err) {
    due = [];
    showError(err);
    render();
    return true;
  }
  selected = Math.min(selected, Math.max(due.length - 1, 0));
  render();
  return due.length > 0 || had === 0;
}

/** Reloads the list, and closes the window once the last note in it is gone. */
async function refresh(keepError = false) {
  if (await load(keepError)) await fit();
  else await appWindow.close();
}

/**
 * Sends `command` for the selected note: marks it done or moves its date on, which takes it off
 * the list.
 */
async function changeNote(command: string, args: Record<string, unknown>) {
  const item = due[selected];
  if (!item) return;
  let failed = false;
  try {
    await invoke(command, { name: item.file, note: item.note, ...args });
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
    void changeNote("set_note_done", { done: true });
  } else if (e.key === "Enter") {
    e.preventDefault();
    void openNote();
  } else if (e.key === "Tab") {
    // Shift+Tab too: there are only two lists.
    e.preventDefault();
    ahead = !ahead;
    selected = 0;
    void load().then(fit);
  } else if (
    !ahead &&
    (e.code === "KeyS" || e.code === "KeyW") &&
    !e.ctrlKey &&
    !e.altKey &&
    !e.metaKey
  ) {
    // By physical key, so any keyboard layout works: S to tomorrow, W a week on.
    e.preventDefault();
    void changeNote("snooze_note", { days: e.code === "KeyS" ? 1 : 7 });
  }
});

// Notes may be done, changed or added anywhere while the reminders are open.
void listen("notes-changed", () => void refresh());

/**
 * Shows the window once it is filled in and fitted: on the notes due, or if there are none (as
 * `/reminders` opens it) on those ahead; unless there are none of those either after all.
 */
async function start() {
  await load();
  if (due.length === 0 && error.hidden) {
    ahead = true;
    await load();
  }
  if (due.length === 0 && error.hidden) {
    await appWindow.close();
    return;
  }
  await fitToContent(MAX_HEIGHT);
  await invoke("reveal_reminders");
}

// Shown only once themed and filled in, so it never flickers.
// The zoom has to be known before the window is fitted to its content.
void Promise.all([followTheme(() => void fit()), followDateFormat(render)]).then(start);
