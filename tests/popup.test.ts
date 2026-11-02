import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { type Call, argsOf, mockTauri, onPlatform } from "./helpers";

// How the popups fit and show themselves; only the Rust side is pretended.
let calls: Call[];

/** A fresh copy of the module, as a new window loads it, on a page of `height` CSS pixels. */
async function load(height: number) {
  calls = mockTauri();
  document.body.innerHTML = "<main></main>";
  Object.defineProperty(document.querySelector("main"), "offsetHeight", { value: height });
  vi.resetModules();
  return import("../src/shared/popup");
}

const commands = () => calls.map((call) => call.cmd);

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
});

afterEach(() => {
  vi.useRealTimers();
  document.body.replaceChildren();
});

test("on Windows a popup is fitted while hidden and only centred and shown after that", async () => {
  const { fitToContent, reveal } = await load(300);
  await fitToContent(400);
  calls.length = 0;
  const shown = reveal();
  await vi.advanceTimersByTimeAsync(1000);
  await shown;
  expect(commands()).toEqual(["center_window", "reveal_window"]);
});

test("on Linux a popup is fitted again up to the same limit once shown, and centred", async () => {
  onPlatform("linux");
  const { fitToContent, reveal } = await load(1000);
  await fitToContent(400);
  calls.length = 0;
  const shown = reveal();
  await vi.advanceTimersByTimeAsync(600);
  await shown;
  expect(commands()).toEqual([
    "center_window",
    "reveal_window",
    "plugin:window|scale_factor",
    "plugin:window|inner_size",
    "plugin:window|set_size",
    "center_window",
  ]);
  expect(JSON.stringify(argsOf(calls, "plugin:window|set_size"))).toContain('"height":400');
});

test("on Linux the fit waits for the page to be laid out in the shown window", async () => {
  onPlatform("linux");
  const { reveal } = await load(300);
  const shown = reveal();
  await vi.advanceTimersByTimeAsync(299);
  expect(commands()).toEqual(["center_window", "reveal_window"]);
  await vi.advanceTimersByTimeAsync(301);
  await shown;
  expect(commands()).toContain("plugin:window|set_size");
});
