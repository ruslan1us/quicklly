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

// The notes with a later date, for Tab.
const AHEAD = [
  {
    file: "inbox.md",
    note: { line: 6, time: "10:00", done: false, text: "dentist 📅 2026-10-14" },
    date: "2026-10-14",
    overdue: -1,
  },
  {
    file: "todo.md",
    note: { line: 2, time: "11:00", done: false, text: "renew passport 📅 2026-10-20" },
    date: "2026-10-20",
    overdue: -7,
  },
  {
    file: "todo.md",
    note: { line: 3, time: "11:30", done: false, text: "book flights 📅 2026-10-20" },
    date: "2026-10-20",
    overdue: -7,
  },
];

let calls: Call[];
let due: typeof DUE;
let ahead: typeof AHEAD;
const rows = () => Array.from(document.querySelectorAll("#rows li"), (li) => li.textContent);
const title = () => document.querySelector("#title")!.textContent;
/** Whether the window was closed. */
const closed = () => calls.some((call) => call.cmd === "plugin:window|close");

/** Opens the reminders window on the notes `due` and `ahead`. */
async function open(dueNotes: typeof DUE, aheadNotes: typeof AHEAD) {
  due = [...dueNotes];
  ahead = [...aheadNotes];
  calls = mockTauri({
    get_appearance: { theme: "default", transparency: 0, scale: 100 },
    get_date_format: "DD/MM/YYYY",
    list_due: (args: { ahead: boolean }) => (args.ahead ? ahead : due),
    // Each change takes the note it was sent for off the list, as the Rust side would.
    set_note_done: (args: { name: string; note: { line: number } }) => drop(args),
    snooze_note: (args: { name: string; note: { line: number } }) => drop(args),
  });
  await loadPage(html, () => import("../src/reminders.ts"));
}

beforeEach(() => open(DUE, AHEAD));

afterEach(() => closePage());

function drop({ name, note }: { name: string; note: { line: number } }) {
  const other = (item: (typeof DUE)[number]) => item.file !== name || item.note.line !== note.line;
  due = due.filter(other);
  ahead = ahead.filter(other);
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

test("Tab shows the notes ahead under their days, and Shift+Tab goes back", async () => {
  expect(title()).toBe("Quicklly · Reminders · 2 due");
  press(document, "Tab");
  await settle();
  expect(title()).toBe("Quicklly · Reminders · 3 ahead");
  expect(rows()).toEqual([
    "── 14/10/2026",
    "❯dentist 📅 14/10/2026tomorrow",
    "── 20/10/2026",
    "renew passport 📅 20/10/2026in 7 days",
    "book flights 📅 20/10/2026in 7 days",
  ]);
  expect(document.querySelector("#help")!.textContent).toContain("Tab due");
  press(document, "Tab", { shiftKey: true });
  await settle();
  expect(title()).toBe("Quicklly · Reminders · 2 due");
  expect(rows()[0]).toBe("❯call back 📅 11/10/20262 days overdue");
});

test("the keys work on the notes ahead too", async () => {
  press(document, "Tab");
  await settle();
  press(document, "ArrowDown");
  press(document, " ");
  await settle();
  expect(argsOf(calls, "set_note_done")).toEqual([
    { name: "todo.md", note: AHEAD[1].note, done: true },
  ]);
  expect(rows()).toEqual([
    "── 14/10/2026",
    "dentist 📅 14/10/2026tomorrow",
    "── 20/10/2026",
    "❯book flights 📅 20/10/2026in 7 days",
  ]);
  // No snoozing ahead: S would bring the note forward.
  press(document, "s", { code: "KeyS" });
  press(document, "w", { code: "KeyW" });
  await settle();
  expect(argsOf(calls, "snooze_note")).toEqual([]);
});

test("the window closes once the last note ahead is done", async () => {
  closePage();
  await open([], [AHEAD[0]]);
  expect(closed()).toBe(false);
  press(document, " ");
  await settle();
  expect(closed()).toBe(true);
});

test("opens on the notes ahead when nothing is due, and closes when there are none", async () => {
  closePage();
  await open([], AHEAD);
  expect(title()).toBe("Quicklly · Reminders · 3 ahead");
  expect(closed()).toBe(false);
  closePage();
  await open([], []);
  expect(closed()).toBe(true);
});

test("Tab to an empty list says so and keeps the window open", async () => {
  closePage();
  await open(DUE, []);
  press(document, "Tab");
  await settle();
  expect(title()).toBe("Quicklly · Reminders · 0 ahead");
  expect(document.querySelector<HTMLElement>("#empty")!.hidden).toBe(false);
  expect(closed()).toBe(false);
});
