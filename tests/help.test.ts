import { afterEach, expect, test } from "vitest";
import { DONE_DAYS } from "../src/shared/dates";
import html from "../src/windows/help/index.html?raw";
import { closePage, loadPage, mockTauri, onPlatform } from "./helpers";

// The help, with its real page and code; only the Rust side is pretended.

/** Opens the help with done notes archived after `days`, or never (null). */
async function open(days: number | null) {
  mockTauri({
    get_appearance: { theme: "default", transparency: 0, scale: 100 },
    "plugin:app|version": "0.8.1",
    get_archive_days: days,
  });
  await loadPage(html, () => import("../src/windows/help/help.ts"));
}

/** The text of the Archive section as shown: hidden parts left out, spaces collapsed. */
function archiveText() {
  const heading = Array.from(document.querySelectorAll("h2")).find(
    (h2) => h2.textContent === "Archive",
  )!;
  const paragraph = heading.nextElementSibling!.cloneNode(true) as HTMLElement;
  for (const hidden of paragraph.querySelectorAll("[hidden]")) hidden.remove();
  return paragraph.textContent!.replace(/\s+/g, " ").trim();
}

afterEach(closePage);

test("the archive text follows the setting", async () => {
  await open(3);
  expect(document.querySelector("header")!.textContent).toBe("Quicklly · Help · v0.8.1");
  expect(archiveText()).toBe(
    "A note marked done gets the day, ✅ 2026-10-13. After 3 days it moves to the same file in " +
      "the archive folder, which the reader and the search leave out. Archive done notes in " +
      "/config sets when, or turns the archive off.",
  );
  closePage();

  await open(1);
  expect(archiveText()).toContain("After 1 day it moves");
  closePage();

  await open(null);
  expect(archiveText()).toBe(
    "A note marked done gets the day, ✅ 2026-10-13. It stays where it is: the archive is off, " +
      "and notes archived before stay in the archive folder, which the reader and the search " +
      "leave out. Archive done notes in /config sets when, or turns the archive off.",
  );
});

test("the done journal's period is the one it lists", async () => {
  await open(3);
  const shown = Array.from(document.querySelectorAll(".done-days"), (el) => el.textContent);
  expect(shown).toEqual([`${DONE_DAYS} days`, `${DONE_DAYS} days`]);
});

const keys = () => Array.from(document.querySelectorAll("dt"), (dt) => dt.textContent!);
const text = () => document.querySelector("main")!.textContent!.replace(/\s+/g, " ");

test("keys and things are named as on Windows", async () => {
  await open(7);
  expect(keys()).toEqual(
    expect.arrayContaining(["Shift+Enter", "Ctrl+E", "Ctrl+V", "Esc, Ctrl+C", "Del", "Ctrl+Z"]),
  );
  expect(keys()).toEqual(expect.arrayContaining(["Ctrl+P", "Ctrl+Enter"]));
  expect(text()).toContain("indent (Shift+Tab: outdent)");
  expect(text()).toContain("move a file to the Recycle Bin");
  expect(text()).toContain('quicklly.exe add "buy milk #todo"');
});

test("on macOS the shortcuts take ⌘, ⌃C closes, ⌫ deletes, files go to the Trash", async () => {
  onPlatform("darwin");
  await open(7);
  expect(keys()).toEqual(expect.arrayContaining(["⇧Enter", "⌘E", "⌘V", "Esc, ⌃C", "⌫", "⌘Z"]));
  expect(keys()).toEqual(expect.arrayContaining(["⌘P", "⌘Enter"]));
  expect(keys().filter((key) => /Ctrl|Shift|Del/.test(key))).toEqual([]);
  expect(text()).toContain("indent (⇧Tab: outdent)");
  expect(text()).toContain("move a file to the Trash");
  expect(text()).toContain('quicklly add "buy milk #todo"');
});

test("on Linux files go to the Trash, the keys are named as on Windows", async () => {
  onPlatform("linux");
  await open(7);
  expect(keys()).toEqual(expect.arrayContaining(["Ctrl+E", "Esc, Ctrl+C", "Del", "Ctrl+Z"]));
  expect(text()).toContain("move a file to the Trash");
  expect(text()).toContain('quicklly add "buy milk #todo"');
});
