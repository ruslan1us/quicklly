import { type MockInstance, afterEach, beforeEach, expect, test, vi } from "vitest";
import html from "../src/windows/done/index.html?raw";
import { type Call, closePage, loadPage, mockTauri, press, settle } from "./helpers";

// The journal of done notes, with its real page and code; only the Rust side is pretended.
const DONE = [
  { file: "inbox.md", text: "call back 📅 2026-10-11", date: "2026-10-14" },
  {
    file: "work/quicklly.md",
    text: "ship it ![Image #1](../images/a.png) https://github.com/ruslan1us/quicklly\nsecond line\n\nafter a blank",
    date: "2026-10-14",
  },
  { file: "archive/todo.md", text: "buy milk", date: "2026-10-09" },
];

let calls: Call[];
let writeText: MockInstance<Clipboard["writeText"]>;
const rows = () => Array.from(document.querySelectorAll("#rows li"), (li) => li.textContent);
const title = () => document.querySelector("#title")!.textContent;
const help = () => document.querySelector("#help")!.textContent;

/** Opens the journal on the notes `done`. */
async function open(done: typeof DONE) {
  calls = mockTauri({
    get_appearance: { theme: "default", transparency: 0, scale: 100 },
    get_date_format: "DD/MM/YYYY",
    list_done: done,
  });
  await loadPage(html, () => import("../src/windows/done/done.ts"));
}

beforeEach(async () => {
  writeText = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue();
  await open(DONE);
});

afterEach(() => {
  closePage();
  vi.restoreAllMocks();
});

test("lists the done notes under their days, first lines only", () => {
  expect(title()).toBe("Quicklly · Done · 3 in 7 days");
  expect(rows()).toEqual([
    "── 14/10/2026",
    "call back 📅 11/10/2026",
    "ship it [Image #1] github.com/rusl...",
    "── 09/10/2026",
    "buy milk",
  ]);
  expect(document.querySelector<HTMLElement>("#empty")!.hidden).toBe(true);
  expect(help()).toBe("C copy · Esc close");
});

test("says when nothing was done", async () => {
  closePage();
  await open([]);
  expect(title()).toBe("Quicklly · Done · 0 in 7 days");
  expect(rows()).toEqual([]);
  const empty = document.querySelector<HTMLElement>("#empty")!;
  expect(empty.hidden).toBe(false);
  expect(empty.textContent).toBe("Nothing done in the last 7 days.");
});

test("C copies the journal as Markdown, on any keyboard layout, and says so", async () => {
  press(document, "с", { code: "KeyC" });
  await settle();
  expect(writeText).toHaveBeenCalledWith(
    [
      "## 14/10/2026",
      "- call back 📅 11/10/2026",
      "- ship it [Image #1] https://github.com/ruslan1us/quicklly",
      "  second line",
      "",
      "  after a blank",
      "## 09/10/2026",
      "- buy milk",
    ].join("\n"),
  );
  expect(help()).toBe("Copied 3 notes · C copy · Esc close");
});

test("Esc closes the window", async () => {
  press(document, "Escape");
  await settle();
  expect(calls.some((call) => call.cmd === "plugin:window|close")).toBe(true);
});
