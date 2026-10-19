import { expect, test, vi } from "vitest";
import { onPlatform } from "./helpers";

/** A fresh copy of `platform.ts`, which reads the platform once, when it loads. */
async function load() {
  vi.resetModules();
  return import("../src/shared/platform");
}

test("the pages are tested as on Windows, whatever the machine", async () => {
  const { platform, isMac, isLinux } = await load();
  expect(platform).toBe("win32");
  expect([isMac, isLinux]).toEqual([false, false]);
});

test("a test can run them as on macOS or Linux", async () => {
  onPlatform("darwin");
  expect(await load()).toMatchObject({ platform: "darwin", isMac: true, isLinux: false });
  onPlatform("linux");
  expect(await load()).toMatchObject({ platform: "linux", isMac: false, isLinux: true });
});

test("a stubbed platform ends with its test", async () => {
  expect((await load()).platform).toBe("win32");
});
