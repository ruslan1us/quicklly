import { afterEach, expect, test, vi } from "vitest";
import themes from "../src/shared/themes.css?raw";
import { onPlatform } from "./helpers";

/** A fresh copy of `theme.ts` with `themes.css` on the page. */
async function load() {
  const style = document.createElement("style");
  style.textContent = themes;
  document.head.append(style);
  vi.resetModules();
  await import("../src/shared/theme");
}

afterEach(() => {
  document.head.replaceChildren();
  delete document.documentElement.dataset.platform;
});

test("on Linux the page rounds the window and draws its edge", async () => {
  onPlatform("linux");
  await load();
  const root = getComputedStyle(document.documentElement);
  const body = getComputedStyle(document.body);
  expect(document.documentElement.dataset.platform).toBe("linux");
  expect(root.backgroundImage).toContain("linear-gradient");
  expect(body.borderRadius).toBe("10px");
  expect(body.boxShadow).toContain("inset");
});

test.each([
  ["Windows", "win32", undefined],
  ["macOS", "darwin", "darwin"],
] as const)("on %s the page leaves the corners to the system", async (_, name, stub) => {
  if (stub) onPlatform(stub);
  await load();
  const root = getComputedStyle(document.documentElement);
  const body = getComputedStyle(document.body);
  expect(document.documentElement.dataset.platform).toBe(name);
  expect(root.backgroundImage).not.toContain("linear-gradient");
  expect(body.borderRadius).toBe("");
  expect(body.boxShadow).toBe("");
});
