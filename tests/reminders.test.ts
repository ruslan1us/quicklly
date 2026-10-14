import { afterEach, beforeEach, expect, test } from "vitest";
import html from "../reminders.html?raw";
import { type Call, argsOf, closePage, loadPage, mockTauri, press, settle } from "./helpers";

// The reminders window, with its real page and code; only the Rust side is pretended.
const DUE = [
  {
    file: "inbox.md",
    note: { line: 4, time: "08:00", done: false, text: "call back 📅 2026-10-11" },
    date: "2026-10-11",
    overdue: 2,
  },
  {
    file: "2026-10-10.md",
    note: { line: 2, time: "09:00", done: false, text: "pay rent 📅 2026-10-13" },
    date: "2026-10-13",
    overdue: 0,
  },
];

let calls: Call[];
let due: typeof DUE;
const rows = () => Array.from(document.querySelectorAll("#rows li"), (li) => li.textContent);
/** Whether the window was closed. */
const closed = () => calls.some((call) => call.cmd === "plugin:window|close");

beforeEach(async () => {
  due = [...DUE];
  calls = mockTauri({
    get_appearance: { theme: "default", transparency: 0, scale: 100 },
    get_date_format: "DD/MM/YYYY",
    list_due: () => due,
    // Each change takes the note it was sent for off the list, as the Rust side would.
    set_note_done: (args: { note: { line: number } }) => drop(args.note.line),
    snooze_note: (args: { note: { line: number } }) => drop(args.note.line),
  });
  await loadPage(html, () => import("../src/reminders.ts"));
});

afterEach(() => closePage());

function drop(line: number) {
  due = due.filter((item) => item.note.line !== line);
}

test("lists the due notes, overdue first", () => {
  expect(rows()).toEqual([
    "❯call back 📅 11/10/20262 days overdue",
    "pay rent 📅 13/10/2026today",
  ]);
});

test("S moves the selected note to tomorrow and takes it off the list", async () => {
  press(document, "s", { code: "KeyS" });
  await settle();
  expect(argsOf(calls, "snooze_note")).toEqual([{ name: "inbox.md", note: DUE[0].note, days: 1 }]);
  expect(rows()).toEqual(["❯pay rent 📅 13/10/2026today"]);
});

test("W moves the selected note a week on, on any keyboard layout", async () => {
  press(document, "ArrowDown");
  press(document, "ц", { code: "KeyW" });
  await settle();
  expect(argsOf(calls, "snooze_note")).toEqual([
    { name: "2026-10-10.md", note: DUE[1].note, days: 7 },
  ]);
  expect(rows()).toEqual(["❯call back 📅 11/10/20262 days overdue"]);
});

test("the window closes once the last note is snoozed, like after Space", async () => {
  press(document, " ");
  await settle();
  expect(closed()).toBe(false);
  press(document, "s", { code: "KeyS" });
  await settle();
  expect(argsOf(calls, "set_note_done")).toEqual([
    { name: "inbox.md", note: DUE[0].note, done: true },
  ]);
  expect(closed()).toBe(true);
});
