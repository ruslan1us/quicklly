import { afterEach, expect, test } from "vitest";
import html from "../src/windows/help/index.html?raw";
import { closePage, loadPage, mockTauri } from "./helpers";

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
