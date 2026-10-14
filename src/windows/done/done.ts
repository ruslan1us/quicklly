import { invoke } from "@tauri-apps/api/core";
import { dateNodes, followDateFormat, shownDate, toDisplay } from "../../shared/dates";
import { fromMarkdown, markdownNodes } from "../../shared/images";
import { linkNodes, shortLink } from "../../shared/links";
import { appWindow, fitToContent, reveal } from "../../shared/popup";
import { followTheme } from "../../shared/theme";

/** A note done in the last days. */
interface Done {
  /** The notes file it is in; `archive/…` once archived. */
  file: string;
  /** Its text, without the day it was done. */
  text: string;
  /** The day it was done, `YYYY-MM-DD`. */
  date: string;
}

const title = document.querySelector<HTMLElement>("#title")!;
const rowsList = document.querySelector<HTMLUListElement>("#rows")!;
const empty = document.querySelector<HTMLParagraphElement>("#empty")!;
const error = document.querySelector<HTMLParagraphElement>("#error")!;
const help = document.querySelector<HTMLElement>("#help")!;

/** Beyond this the list scrolls instead of the window growing. */
const MAX_HEIGHT = 400;
const SCROLL_STEP = 44;

/** The notes done, the latest day first. */
let done: Done[] = [];

/** A short answer shown in front of the help, like "Copied 3 notes". */
let message = "";
let messageTimer: number | undefined;

const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

function span(text: string, className = "") {
  const el = document.createElement("span");
  el.textContent = text;
  el.className = className;
  return el;
}

/**
 * A row with the first line of the note's text, as the reminders show it: links short and
 * underlined, images as `[Image #N]` labels and dates in the date format.
 */
function noteRow(text: string) {
  const li = document.createElement("li");
  li.className = "note";
  const other = (part: string) => dateNodes(part, (rest) => linkNodes(rest, shortLink));
  li.append(...markdownNodes(toDisplay(text.split("\n")[0]), other));
  return li;
}

/** The line above the notes done on one day: its date. */
function dayRow(date: string) {
  const li = document.createElement("li");
  li.className = "day";
  li.textContent = `── ${shownDate(date)}`;
  return li;
}

function renderHelp() {
  help.textContent = "C copy · Esc close";
  if (message) help.prepend(span(message, "message"), " · ");
}

function render() {
  title.textContent = `Quicklly · Done · ${done.length} in 7 days`;
  empty.hidden = done.length > 0;
  const rows: HTMLLIElement[] = [];
  done.forEach((item, i) => {
    if (item.date !== done[i - 1]?.date) rows.push(dayRow(item.date));
    rows.push(noteRow(item.text));
  });
  rowsList.replaceChildren(...rows);
  renderHelp();
}

/**
 * The journal as Markdown, for a report: a heading with each day's date, the notes done that
 * day under it, their images as labels and their dates in the date format.
 */
function markdown() {
  const lines: string[] = [];
  done.forEach((item, i) => {
    if (item.date !== done[i - 1]?.date) lines.push(`## ${shownDate(item.date)}`);
    const text = fromMarkdown(toDisplay(item.text)).text;
    // Extra lines are indented under the first one, like in the notes files.
    lines.push(`- ${text.replace(/\n(?=[^\n])/g, "\n  ")}`);
  });
  return lines.join("\n");
}

/**
 * Shows `message` in front of the help for two seconds; the window is refitted in case it
 * takes a line more.
 */
function showMessage(text: string) {
  window.clearTimeout(messageTimer);
  message = text;
  renderHelp();
  void fitToContent(MAX_HEIGHT);
  messageTimer = window.setTimeout(() => {
    message = "";
    renderHelp();
    void fitToContent(MAX_HEIGHT);
  }, 2000);
}

/** Copies the journal as Markdown, and says how many notes in the footer. */
async function copy() {
  if (done.length === 0) return;
  try {
    await navigator.clipboard.writeText(markdown());
  } catch (err) {
    showError(err);
    return;
  }
  showMessage(`Copied ${plural(done.length, "note")}`);
}

function showError(err: unknown) {
  error.textContent = String(err);
  error.hidden = false;
}

document.addEventListener("keydown", (e) => {
  // Esc or Ctrl+C (by physical key, so any keyboard layout works) closes like in a terminal.
  if (e.key === "Escape" || (e.ctrlKey && e.code === "KeyC")) {
    e.preventDefault();
    void appWindow.close();
  } else if (e.key === "ArrowUp" || e.key === "ArrowDown") {
    e.preventDefault();
    // The body is what scrolls (see terminal.css).
    document.body.scrollBy(0, e.key === "ArrowUp" ? -SCROLL_STEP : SCROLL_STEP);
  } else if (e.key === "PageUp" || e.key === "PageDown") {
    e.preventDefault();
    // A page, keeping a line of the last one in view.
    const page = document.body.clientHeight - SCROLL_STEP;
    document.body.scrollBy(0, e.key === "PageUp" ? -page : page);
  } else if (e.code === "KeyC" && !e.altKey && !e.metaKey) {
    // By physical key, so any keyboard layout works.
    e.preventDefault();
    void copy();
  }
});

async function load() {
  try {
    done = await invoke<Done[]>("list_done");
  } catch (err) {
    showError(err);
  }
  render();
  await fitToContent(MAX_HEIGHT);
}

// Shown only once themed and filled in, so it never flickers.
// The zoom has to be known before the window is fitted to its content.
void Promise.all([followTheme(() => void fitToContent(MAX_HEIGHT)), followDateFormat(render)])
  .then(load)
  .then(reveal);
