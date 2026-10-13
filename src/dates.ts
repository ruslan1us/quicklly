import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/**
 * Dates in notes. Files keep them as Obsidian Tasks due dates, `📅 2026-10-15`; text fields,
 * the reader and the search results show them in the date format of the settings,
 * `📅 15/10/2026`, and every text that leaves a field has them turned back.
 */
export type DateFormat = "DD/MM/YYYY" | "MM/DD/YYYY" | "YYYY-MM-DD" | "DD.MM.YYYY";

/** The date format of the settings; kept up to date by `followDateFormat`. */
export let dateFormat: DateFormat = "DD/MM/YYYY";

const MARK = "📅 ";
/** A date as files store it. */
const STORED = /📅 (\d{4})-(\d{2})-(\d{2})(?!\d)/gu;

/** The parts of `format` in order, e.g. `["DD", "MM", "YYYY"]`. */
const parts = (format: DateFormat) => format.match(/DD|MM|YYYY/g)!;

/** A regular expression source for a date in `format`; day and month may have one digit. */
function pattern(format: DateFormat) {
  return format.replace(/DD|MM|YYYY|[./-]/g, (part) =>
    part === "YYYY" ? "(\\d{4})" : part.length === 2 ? "(\\d{1,2})" : part === "." ? "\\." : part,
  );
}

/** A date as fields show it, in `format`. */
const shown = (format: DateFormat) => new RegExp(`${MARK}${pattern(format)}(?!\\d)`, "gu");

/** The local date of `year`, `month` (1–12) and `day`, or null when there is no such day. */
function makeDate(year: number, month: number, day: number) {
  const date = new Date(year, month - 1, day);
  const real =
    date.getFullYear() === year && date.getMonth() === month - 1 && date.getDate() === day;
  return real ? date : null;
}

/** The date of the regular expression groups `values`, written in `format`. */
function fromParts(format: DateFormat, values: string[]) {
  const value = (part: string) => Number(values[parts(format).indexOf(part)]);
  return makeDate(value("YYYY"), value("MM"), value("DD"));
}

const pad = (n: number) => String(n).padStart(2, "0");

/** `date` written in `format`. */
export function formatDate(date: Date, format = dateFormat) {
  return format
    .replace("DD", pad(date.getDate()))
    .replace("MM", pad(date.getMonth() + 1))
    .replace("YYYY", String(date.getFullYear()));
}

/** The date `text` stands for in `format`, or null if it is not a real date in it. */
export function parseDate(text: string, format = dateFormat) {
  const match = new RegExp(`^${pattern(format)}$`).exec(text);
  return match ? fromParts(format, match.slice(1)) : null;
}

/** The day after `today`. */
export function tomorrow(today = new Date()) {
  return new Date(today.getFullYear(), today.getMonth(), today.getDate() + 1);
}

/** `date` as it is put into a text field: `📅 15/10/2026`. */
export const dateLabel = (date: Date, format = dateFormat) => MARK + formatDate(date, format);

/** `text` with each date shown in `format` stored as `📅 YYYY-MM-DD`; other text stays. */
export function toStored(text: string, format = dateFormat) {
  return text.replace(shown(format), (typed, ...groups: string[]) => {
    const date = fromParts(format, groups.slice(0, 3));
    return date ? dateLabel(date, "YYYY-MM-DD") : typed;
  });
}

/** `text` with each stored `📅 YYYY-MM-DD` shown in `format`; other text stays. */
export function toDisplay(text: string, format = dateFormat) {
  return text.replace(STORED, (stored, year: string, month: string, day: string) => {
    const date = makeDate(Number(year), Number(month), Number(day));
    return date ? dateLabel(date, format) : stored;
  });
}

/**
 * `/date` typed in a text, as a word of its own, with the word after it if there is one:
 * `/date`, `/date tomorrow`, `/date 15/10/2026`.
 */
const COMMAND = /(?<=^|\s)\/date(?:[ \t]+(\S+))?(?=\s|$)/;

/** The first `/date` in a text: where it starts and ends, and the word after it. */
export interface DateCommand {
  start: number;
  end: number;
  arg?: string;
}

export function findDateCommand(text: string): DateCommand | null {
  const match = COMMAND.exec(text);
  if (!match) return null;
  return { start: match.index, end: match.index + match[0].length, arg: match[1] };
}

/** The date a `/date` word stands for: `tomorrow`, or a date in `format`; null otherwise. */
export function commandDate(arg: string, today = new Date(), format = dateFormat) {
  return arg.toLowerCase() === "tomorrow" ? tomorrow(today) : parseDate(arg, format);
}

const plain = (text: string): Node[] => [document.createTextNode(text)];

/**
 * `text` (with dates shown in the date format) as nodes: each date in an accent `.date` span
 * (its content made by `inner`), the text around them made by `other`.
 */
export function dateNodes(text: string, other = plain, inner = plain): Node[] {
  const nodes: Node[] = [];
  let from = 0;
  for (const match of text.matchAll(shown(dateFormat))) {
    if (!fromParts(dateFormat, match.slice(1, 4))) continue;
    if (match.index > from) nodes.push(...other(text.slice(from, match.index)));
    const span = document.createElement("span");
    span.className = "date";
    span.append(...inner(match[0]));
    nodes.push(span);
    from = match.index + match[0].length;
  }
  if (from < text.length) nodes.push(...other(text.slice(from)));
  return nodes;
}

/**
 * Makes a date in `field` go as a whole: Backspace right after it or Delete right before it
 * removes all of `📅 15/10/2026` (Ctrl+Z brings it back).
 */
export function deleteDatesWhole(field: HTMLTextAreaElement) {
  field.addEventListener("keydown", (e) => {
    const back = e.key === "Backspace";
    if ((!back && e.key !== "Delete") || e.shiftKey || e.ctrlKey || e.altKey || e.metaKey) return;
    const { selectionStart: caret, selectionEnd, value } = field;
    if (caret !== selectionEnd) return;
    for (const match of value.matchAll(shown(dateFormat))) {
      const end = match.index + match[0].length;
      if ((back ? end : match.index) !== caret) continue;
      if (!fromParts(dateFormat, match.slice(1, 4))) continue;
      e.preventDefault();
      field.setSelectionRange(match.index, end);
      document.execCommand("delete");
      return;
    }
  });
}

/**
 * Loads the date format of the settings, and follows it when it is changed there; `onChange`
 * runs after a change with the format before it, e.g. to show a text's dates in the new one.
 */
export async function followDateFormat(onChange?: (previous: DateFormat) => void) {
  await listen<DateFormat>("date-format-changed", (event) => {
    const previous = dateFormat;
    dateFormat = event.payload;
    onChange?.(previous);
  });
  dateFormat = await invoke<DateFormat>("get_date_format");
}
