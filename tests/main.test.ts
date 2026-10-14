import { afterEach, beforeEach, expect, test, vi } from "vitest";
import html from "../src/windows/input/index.html?raw";
import { type Call, argsOf, closePage, loadPage, mockTauri, press, settle, type } from "./helpers";

// The input window, with its real page and code; only the Rust side is pretended.
let calls: Call[];
let note: HTMLTextAreaElement;
let calendar: HTMLDivElement;

/**
 * Opens the input window, with dates in `dateFormat`, on 13 October 2026; other commands are
 * answered from `replies`.
 */
async function open(dateFormat = "DD/MM/YYYY", replies: Record<string, unknown> = {}) {
  vi.setSystemTime(new Date(2026, 9, 13, 15, 40));
  calls = mockTauri({
    get_appearance: { theme: "default", transparency: 0, scale: 100 },
    get_date_format: dateFormat,
    get_update: null,
    ...replies,
  });
  await loadPage(html, () => import("../src/windows/input/main.ts"));
  note = document.querySelector("#note")!;
  calendar = document.querySelector("#calendar")!;
}

/** Whether the window was hidden. */
const hidden = () => calls.some((call) => call.cmd === "plugin:window|hide");

beforeEach(() => open());

afterEach(() => {
  closePage();
  vi.useRealTimers();
});

test("Enter saves the note and hides the window", async () => {
  type(note, "call mom #todo");
  press(note, "Enter");
  await settle();
  expect(argsOf(calls, "save_note")).toEqual([{ text: "call mom #todo" }]);
  expect(hidden()).toBe(true);
  expect(note.value).toBe("");
});

test("Shift+Enter starts a new line instead", async () => {
  type(note, "call mom");
  expect(press(note, "Enter", { shiftKey: true }).defaultPrevented).toBe(false);
  await settle();
  expect(argsOf(calls, "save_note")).toEqual([]);
});

test("Enter turns /date and a date into the date first, saved as YYYY-MM-DD", async () => {
  type(note, "call mom /date 15/10/2026");
  press(note, "Enter");
  await settle();
  expect(note.value).toBe("call mom 📅 15/10/2026 ");
  expect(argsOf(calls, "save_note")).toEqual([]);

  press(note, "Enter");
  await settle();
  expect(argsOf(calls, "save_note")).toEqual([{ text: "call mom 📅 2026-10-15" }]);
});

test("/date and Enter opens the calendar instead of saving; Enter picks a day", async () => {
  type(note, "call mom /date");
  press(note, "Enter");
  await settle();
  expect(calendar.hidden).toBe(false);
  expect(argsOf(calls, "save_note")).toEqual([]);

  press(note, "ArrowRight");
  press(note, "Enter");
  expect(calendar.hidden).toBe(true);
  expect(note.value).toBe("call mom 📅 14/10/2026 ");
  press(note, "Enter");
  await settle();
  expect(argsOf(calls, "save_note")).toEqual([{ text: "call mom 📅 2026-10-14" }]);
});

test("Esc closes the calendar but not the window", async () => {
  type(note, "call mom /date");
  press(note, "Enter");
  press(note, "Escape");
  await settle();
  expect(calendar.hidden).toBe(true);
  expect(hidden()).toBe(false);
  expect(note.value).toBe("call mom /date");
});

test("dates are typed in the date format of the settings", async () => {
  closePage();
  await open("MM/DD/YYYY");
  type(note, "call mom /date 10/15/2026");
  press(note, "Enter");
  press(note, "Enter");
  await settle();
  expect(argsOf(calls, "save_note")).toEqual([{ text: "call mom 📅 2026-10-15" }]);
});

test("search results mark the query in the text as shown, dates and image labels too", async () => {
  closePage();
  const hit = {
    file: "todo.md",
    line: 2,
    day: "2026-10-13",
    time: "09:00",
    done: false,
    text: "bank 📅 2026-10-15 ![Image #1](images/a.png)",
  };
  await open("DD/MM/YYYY", { search_notes: [hit] });
  const marked = () => [...document.querySelectorAll("#results mark")].map((m) => m.textContent);

  type(note, "?");
  type(note, "15/10/2026");
  await settle();
  expect(argsOf(calls, "search_notes")).toContainEqual({ query: "15/10/2026" });
  expect(document.querySelector("#results .text")!.textContent).toBe(
    "bank 📅 15/10/2026 [Image #1]",
  );
  expect(marked()).toEqual(["15/10/2026"]);

  note.select();
  type(note, "image #1");
  await settle();
  expect(marked()).toEqual(["Image #1"]);
});
