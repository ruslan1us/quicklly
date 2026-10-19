import { afterEach, expect, test, vi } from "vitest";
import html from "../src/windows/settings/index.html?raw";
import { closePage, loadPage, mockTauri, onPlatform, press } from "./helpers";

// The settings, with their real page and code; only the Rust side is pretended.
const SETTINGS = {
  notesDir: "C:\\notes",
  notesDirIsDefault: true,
  mode: "daily",
  hotkey: "Ctrl+Alt+N",
  hotkeyIsDefault: true,
  theme: "default",
  autostart: false,
  autoUpdate: true,
  transparency: 30,
  scale: 100,
  hotkeyTarget: "input",
  padPosition: "topRight",
  dateFormat: "DD/MM/YYYY",
  archiveAfter: "sevenDays",
};

const rows = () => Array.from(document.querySelectorAll("#rows li"), (li) => li.textContent);
const help = () => document.querySelector("#help")!.textContent;

async function open() {
  mockTauri({
    get_appearance: { theme: "default", transparency: 0, scale: 100 },
    get_settings: SETTINGS,
  });
  await loadPage(html, () => import("../src/windows/settings/settings.ts"));
  await vi.waitFor(() => expect(rows()).not.toEqual([]));
}

/** Starts recording the hotkey (the first row) and holds Ctrl, Alt, Shift and the Super key. */
function holdModifiers() {
  press(document, "Enter");
  press(document, "Meta", {
    code: "MetaLeft",
    ctrlKey: true,
    altKey: true,
    shiftKey: true,
    metaKey: true,
  });
  return rows()[0];
}

afterEach(closePage);

test("on Windows: Start with Windows, Del to reset, the Win key", async () => {
  await open();
  expect(rows()).toContain("Start with Windows‹ Off ›");
  expect(help()).toBe("↑↓ select · ←→ change · Enter edit · Del reset · Esc close");
  expect(holdModifiers()).toBe("❯HotkeyCtrl+Alt+Shift+Win+…");
});

test("on macOS: Start at login, ⌫ to reset, ⌃⌥⇧⌘", async () => {
  onPlatform("darwin");
  await open();
  expect(rows()).toContain("Start at login‹ Off ›");
  expect(help()).toBe("↑↓ select · ←→ change · Enter edit · ⌫ reset · Esc close");
  expect(holdModifiers()).toBe("❯Hotkey⌃⌥⇧⌘…");
});

test("on Linux: Start at login, Del to reset, the Super key", async () => {
  onPlatform("linux");
  await open();
  expect(rows()).toContain("Start at login‹ Off ›");
  expect(help()).toBe("↑↓ select · ←→ change · Enter edit · Del reset · Esc close");
  expect(holdModifiers()).toBe("❯HotkeyCtrl+Alt+Shift+Super+…");
});
