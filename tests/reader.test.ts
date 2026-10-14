import { type MockInstance, afterEach, beforeEach, expect, test, vi } from "vitest";
import html from "../src/windows/reader/index.html?raw";
import { type Call, argsOf, closePage, loadPage, mockTauri, press, settle } from "./helpers";

// The reader, with its real page and code; only the Rust side is pretended.
const FILES = [
  { name: "2026-10-13.md", notes: 2, done: 1, changed: 2 },
  { name: "inbox.md", notes: 0, done: 0, changed: 1 },
];
const IMAGE = "images/2026-10-13_09-15-00.png";
const LINK = "https://github.com/ruslan1us/quicklly/releases";
const NOTES = [
  {
    kind: "note",
    line: 1,
    time: "09:15",
    done: true,
    text: `milk ![Image #1](${IMAGE}) ✅ 2026-10-13`,
  },
  { kind: "note", line: 2, time: "14:32", done: false, text: `read ${LINK} 📅 2026-10-15` },
];

let calls: Call[];
let writeText: MockInstance<Clipboard["writeText"]>;
const text = (selector: string) => document.querySelector(selector)?.textContent;
const rows = () => Array.from(document.querySelectorAll("#rows li"), (li) => li.textContent);

/** Opens the reader, its Rust side answering as below unless `replies` say otherwise. */
async function open(replies: Record<string, unknown> = {}) {
  calls = mockTauri({
    get_appearance: { theme: "default", transparency: 0, scale: 100 },
    get_date_format: "DD/MM/YYYY",
    get_reader_pin: { pinned: false, file: null },
    get_reader_sort: "recent",
    take_reader_target: null,
    list_note_files: FILES,
    read_note_file: NOTES,
    get_archive_days: 7,
    ...replies,
  });
  await loadPage(html, () => import("../src/windows/reader/reader.ts"));
}

beforeEach(async () => {
  vi.setSystemTime(new Date(2026, 9, 13, 15, 40));
  writeText = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue();
  await open();
});

afterEach(() => {
  closePage();
  vi.restoreAllMocks();
  vi.useRealTimers();
});

/** Opens the first file, today's, with Enter. */
async function openToday() {
  press(document, "Enter");
  await settle();
}

test("lists the files, the latest changed first", () => {
  expect(text("#title")).toBe("Quicklly · Notes · recent first");
  expect(rows()).toEqual(["❯2026-10-13today▰▰▰▰▱▱▱▱1/2", "inbox.mdempty"]);
});

test("Enter opens a file's notes, with the latest one selected", async () => {
  await openToday();
  expect(argsOf(calls, "read_note_file")).toEqual([{ name: "2026-10-13.md" }]);
  expect(text("#title")).toBe("Quicklly · 2026-10-13 · 2 notes · 1 done");
  // Links short, dates in the date format, images as labels; the done day gives way to the
  // time left before the archive.
  expect(rows()).toEqual([
    "09:15✓milk [Image #1]archive in 7 days",
    "❯14:32read github.com/rusl... 📅 15/10/2026",
  ]);
});

test("C copies the note's links whole and says how many", async () => {
  await openToday();
  press(document, "c", { code: "KeyC" });
  await settle();
  expect(writeText).toHaveBeenCalledWith(LINK);
  expect(text("#help")).toMatch(/^Copied 1 link · /);
});

test("C on a note without links says so", async () => {
  await openToday();
  press(document, "ArrowUp");
  press(document, "c", { code: "KeyC" });
  await settle();
  expect(writeText).not.toHaveBeenCalled();
  expect(text("#help")).toMatch(/^No links in this note · /);
});

test("O opens the note's images and says how many", async () => {
  await openToday();
  press(document, "ArrowUp");
  press(document, "o", { code: "KeyO" });
  await settle();
  expect(argsOf(calls, "open_images")).toEqual([{ name: "2026-10-13.md", paths: [IMAGE] }]);
  expect(text("#help")).toMatch(/^Opened 1 image · /);
});

test("O on a note without images says so", async () => {
  await openToday();
  press(document, "o", { code: "KeyO" });
  await settle();
  expect(argsOf(calls, "open_images")).toEqual([]);
  expect(text("#help")).toMatch(/^No images in this note · /);
});

// Files in folders, as `list_note_files` gives them: by name, as a tree.
const NESTED = [
  { name: "inbox.md", notes: 0, done: 0, changed: 1 },
  { name: "todo.md", notes: 0, done: 0, changed: 5 },
  { name: "2026-10-13.md", notes: 0, done: 0, changed: 2 },
  { name: "work/quicklly.md", notes: 0, done: 0, changed: 4 },
  { name: "work/deep/x.md", notes: 0, done: 0, changed: 3 },
  { name: "zeta/a.md", notes: 0, done: 0, changed: 6 },
];

/** Opens the reader again on the files in folders, sorted as `sort`. */
async function openNested(sort: string) {
  closePage();
  await open({ get_reader_sort: sort, list_note_files: NESTED });
}

test("by name, files in folders show as a tree that ↑↓ go through file by file", async () => {
  await openNested("name");
  expect(rows()).toEqual([
    "❯inbox.mdempty",
    "todo.mdempty",
    "2026-10-13todayempty",
    "work/",
    "quicklly.mdempty",
    "deep/",
    "x.mdempty",
    "zeta/",
    "a.mdempty",
  ]);
  const folders = Array.from(document.querySelectorAll<HTMLElement>("#rows .folder"));
  expect(folders.map((li) => li.textContent)).toEqual(["work/", "deep/", "zeta/"]);
  const indents = folders.map(
    (li) => li.querySelector<HTMLElement>("span:last-child")!.style.paddingLeft,
  );
  expect(indents).toEqual(["0ch", "2ch", "0ch"]);
  for (let i = 0; i < 3; i++) press(document, "ArrowDown");
  expect(rows()[4]).toBe("❯quicklly.mdempty");
  press(document, "ArrowDown");
  expect(rows()[6]).toBe("❯x.mdempty");
  press(document, "Enter");
  await settle();
  expect(argsOf(calls, "read_note_file")).toEqual([{ name: "work/deep/x.md" }]);
  expect(text("#title")).toBe("Quicklly · work/deep/x.md · 2 notes · 1 done");
  press(document, "ArrowUp");
  press(document, "o", { code: "KeyO" });
  await settle();
  expect(argsOf(calls, "open_images")).toEqual([{ name: "work/deep/x.md", paths: [IMAGE] }]);
});

test("recent first, files in folders show their whole path", async () => {
  await openNested("recent");
  expect(rows()).toEqual([
    "❯zeta/a.mdempty",
    "todo.mdempty",
    "work/quicklly.mdempty",
    "work/deep/x.mdempty",
    "2026-10-13todayempty",
    "inbox.mdempty",
  ]);
});

test("Tab keeps a file in a folder selected in its new place", async () => {
  await openNested("name");
  for (let i = 0; i < 3; i++) press(document, "ArrowDown");
  press(document, "Tab");
  await settle();
  expect(rows()[2]).toBe("❯work/quicklly.mdempty");
  press(document, "Tab");
  await settle();
  expect(rows()[4]).toBe("❯quicklly.mdempty");
});
