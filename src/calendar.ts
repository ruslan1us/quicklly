import { commandDate, dateFormat, dateLabel, findDateCommand } from "./dates";

const MONTHS = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];
const WEEKDAYS = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
/** By `Date.getDay()`, Sunday first. */
const WEEKDAY_NAMES = [
  "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday",
];
const DAY_MS = 24 * 60 * 60 * 1000;

/** How far `date` is from `today`: "today", "tomorrow", "in 5 days", "3 days ago". */
function fromToday(date: Date, today: Date) {
  // Midnights in UTC, so a change to or from summer time doesn't shift the count.
  const utc = (d: Date) => Date.UTC(d.getFullYear(), d.getMonth(), d.getDate());
  const days = Math.round((utc(date) - utc(today)) / DAY_MS);
  if (days === 0) return "today";
  if (days === 1) return "tomorrow";
  if (days === -1) return "yesterday";
  return days > 0 ? `in ${days} days` : `${-days} days ago`;
}
/** Keys that move the selected day, by how many days. */
const DAY_STEPS: Record<string, number> = {
  ArrowLeft: -1,
  ArrowRight: 1,
  ArrowUp: -7,
  ArrowDown: 7,
};
/** Keys that move the selected day by a month. */
const MONTH_STEPS: Record<string, number> = { PageUp: -1, PageDown: 1 };
/** Pressed alone, these don't type anything, so they leave the calendar open. */
const MODIFIERS = new Set(["Shift", "Control", "Alt", "Meta"]);

export interface DatePicker {
  /**
   * Enter: turns the first `/date` in the text into a date, opens the calendar for a bare
   * `/date`, or reports a word after it that is not a date. Returns false when there is none.
   */
  resolve(): boolean;
  /** Handles the calendar's keys while it is shown; returns true when the key was used. */
  keydown(e: KeyboardEvent): boolean;
  /** Hides the calendar; the text stays as it is. */
  close(): void;
}

const sameDay = (a: Date, b: Date) =>
  a.getFullYear() === b.getFullYear() &&
  a.getMonth() === b.getMonth() &&
  a.getDate() === b.getDate();

function span(text: string, className: string) {
  const el = document.createElement("span");
  el.textContent = text;
  el.className = className;
  return el;
}

/**
 * `/date` in `input`: picks a date in a calendar shown in `element`, the same in every window,
 * and types it over the `/date`. `onChange` runs after the calendar is shown, changed or
 * hidden, to fit the window to it; `onError` gets the message for a word that is not a date.
 */
export function datePicker(
  input: HTMLTextAreaElement,
  element: HTMLElement,
  { onChange = () => {}, onError }: { onChange?: () => void; onError: (message: string) => void },
): DatePicker {
  /** The day picked in the calendar; null while it is hidden. */
  let selected: Date | null = null;

  function render() {
    element.hidden = selected === null;
    if (!selected) {
      element.replaceChildren();
      onChange();
      return;
    }
    const year = selected.getFullYear();
    const month = selected.getMonth();
    const today = new Date();
    const title = span(`${MONTHS[month]} ${year}`, "month");
    const days = document.createElement("div");
    days.className = "days";
    days.append(...WEEKDAYS.map((name) => span(name, "weekday")));
    // Weeks start on Monday; the days before the 1st are left blank.
    const blanks = (new Date(year, month, 1).getDay() + 6) % 7;
    days.append(...Array.from({ length: blanks }, () => span("", "")));
    const count = new Date(year, month + 1, 0).getDate();
    for (let day = 1; day <= count; day++) {
      const date = new Date(year, month, day);
      const cell = span(String(day), "day");
      cell.classList.toggle("today", sameDay(date, today));
      cell.classList.toggle("selected", day === selected.getDate());
      // mousedown, so the text keeps the focus.
      cell.addEventListener("mousedown", (e) => {
        e.preventDefault();
        pick(date);
      });
      days.append(cell);
    }
    const sheet = document.createElement("div");
    sheet.append(title, days);
    // Beside the month: the picked day in words, how far it is, and the keys.
    const picked = `${WEEKDAY_NAMES[selected.getDay()]}, ${selected.getDate()} ${MONTHS[month]}`;
    const info = document.createElement("div");
    info.className = "info";
    info.append(
      span(`${picked} ${year}`, "picked"),
      span(fromToday(selected, today), "distance"),
      span("←→ day · ↑↓ week", "keys"),
      span("PgUp PgDn month", "keys"),
      span("Enter pick · Esc close", "keys"),
    );
    const layout = document.createElement("div");
    layout.className = "layout";
    layout.append(sheet, info);
    element.replaceChildren(layout);
    onChange();
  }

  function close() {
    if (!selected) return;
    selected = null;
    render();
  }

  /**
   * Types `date` over the first `/date` (and the word after it), keeping Ctrl+Z working, with
   * the caret after it and a space, to go on writing.
   */
  function pick(date: Date) {
    selected = null;
    render();
    const command = findDateCommand(input.value);
    if (!command) return;
    const spaced = /\s/.test(input.value[command.end] ?? "");
    input.setSelectionRange(command.start, command.end);
    document.execCommand("insertText", false, dateLabel(date) + (spaced ? "" : " "));
    // A space already after it: the caret goes past it.
    if (spaced) input.setSelectionRange(input.selectionEnd + 1, input.selectionEnd + 1);
  }

  function resolve() {
    const command = findDateCommand(input.value);
    if (!command) return false;
    if (command.arg === undefined) {
      selected = new Date();
      render();
      return true;
    }
    const date = commandDate(command.arg);
    if (date) pick(date);
    else onError(`Unknown date: ${command.arg} — use ${dateFormat} or tomorrow`);
    return true;
  }

  function keydown(e: KeyboardEvent) {
    if (!selected) return false;
    const plain = !e.shiftKey && !e.ctrlKey && !e.altKey && !e.metaKey;
    const day = selected.getDate();
    if (plain && e.key in DAY_STEPS) {
      selected = new Date(selected.getFullYear(), selected.getMonth(), day + DAY_STEPS[e.key]);
      render();
    } else if (plain && e.key in MONTH_STEPS) {
      // The same day of the other month, or its last day if it is shorter.
      const month = selected.getMonth() + MONTH_STEPS[e.key];
      const last = new Date(selected.getFullYear(), month + 1, 0).getDate();
      selected = new Date(selected.getFullYear(), month, Math.min(day, last));
      render();
    } else if (plain && e.key === "Enter") {
      pick(selected);
    } else if (plain && e.key === "Escape") {
      close();
    } else {
      // Anything else closes the calendar and does what it usually does.
      if (!MODIFIERS.has(e.key)) close();
      return false;
    }
    e.preventDefault();
    return true;
  }

  input.addEventListener("input", close);
  return { resolve, keydown, close };
}
