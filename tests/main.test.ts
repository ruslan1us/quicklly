import { afterEach, beforeEach, expect, test, vi } from "vitest";
import html from "../src/windows/input/index.html?raw";
import { type Call, argsOf, closePage, loadPage, mockTauri, press, settle, type } from "./helpers";

// The input window, with its real page and code; only the Rust side is pretended.
let calls: Call[];
let note: HTMLTextAreaElement;
let calendar: HTMLDivElement;

/**
 * Opens the input window, with dates in `dateFormat`, on 13 October 2026; other commands are
 * answered from `replies`.
 */
async function open(dateFormat = "DD/MM/YYYY", replies: Record<string, unknown> = {}) {
  vi.setSystemTime(new Date(2026, 9, 13, 15, 40));
  calls = mockTauri({
    get_appearance: { theme: "default", transparency: 0, scale: 100 },
    get_date_format: dateFormat,
    get_update: null,
    ...replies,
  });
  await loadPage(html, () => import("../src/windows/input/main.ts"));
  note = document.querySelector("#note")!;
  calendar = document.querySelector("#calendar")!;
}

/** Whether the window was hidden. */
const hidden = () => calls.some((call) => call.cmd === "plugin:window|hide");

beforeEach(() => open());

afterEach(() => {
  closePage();
  vi.useRealTimers();
});

test("Enter saves the note and hides the window", async () => {
  type(note, "call mom #todo");
  press(note, "Enter");
  await settle();
  expect(argsOf(calls, "save_note")).toEqual([{ text: "call mom #todo" }]);
  expect(hidden()).toBe(true);
  expect(note.value).toBe("");
});

test("Shift+Enter starts a new line instead", async () => {
  type(note, "call mom");
  expect(press(note, "Enter", { shiftKey: true }).defaultPrevented).toBe(false);
  await settle();
  expect(argsOf(calls, "save_note")).toEqual([]);
});

test("Enter turns /date and a date into the date first, saved as YYYY-MM-DD", async () => {
  type(note, "call mom /date 15/10/2026");
  press(note, "Enter");
  await settle();
  expect(note.value).toBe("call mom 📅 15/10/2026 ");
  expect(argsOf(calls, "save_note")).toEqual([]);

  press(note, "Enter");
  await settle();
  expect(argsOf(calls, "save_note")).toEqual([{ text: "call mom 📅 2026-10-15" }]);
});

test("/date and Enter opens the calendar instead of saving; Enter picks a day", async () => {
  type(note, "call mom /date");
  press(note, "Enter");
  await settle();
  expect(calendar.hidden).toBe(false);
  expect(argsOf(calls, "save_note")).toEqual([]);

  press(note, "ArrowRight");
  press(note, "Enter");
  expect(calendar.hidden).toBe(true);
  expect(note.value).toBe("call mom 📅 14/10/2026 ");
  press(note, "Enter");
  await settle();
  expect(argsOf(calls, "save_note")).toEqual([{ text: "call mom 📅 2026-10-14" }]);
});

test("Esc closes the calendar but not the window", async () => {
  type(note, "call mom /date");
  press(note, "Enter");
  press(note, "Escape");
  await settle();
  expect(calendar.hidden).toBe(true);
  expect(hidden()).toBe(false);
  expect(note.value).toBe("call mom /date");
});

test("dates are typed in the date format of the settings", async () => {
  closePage();
  await open("MM/DD/YYYY");
  type(note, "call mom /date 10/15/2026");
  press(note, "Enter");
  press(note, "Enter");
  await settle();
  expect(argsOf(calls, "save_note")).toEqual([{ text: "call mom 📅 2026-10-15" }]);
});

test("search results mark the query in the text as shown, dates and image labels too", async () => {
  closePage();
  const hit = {
    file: "todo.md",
    line: 2,
    day: "2026-10-13",
    time: "09:00",
    done: false,
    text: "bank 📅 2026-10-15 ![Image #1](images/a.png)",
  };
  await open("DD/MM/YYYY", { search_notes: [hit] });
  const marked = () => [...document.querySelectorAll("#results mark")].map((m) => m.textContent);

  type(note, "?");
  type(note, "15/10/2026");
  await settle();
  expect(argsOf(calls, "search_notes")).toContainEqual({ query: "15/10/2026" });
  expect(document.querySelector("#results .text")!.textContent).toBe(
    "bank 📅 15/10/2026 [Image #1]",
  );
  expect(marked()).toEqual(["15/10/2026"]);

  note.select();
  type(note, "image #1");
  await settle();
  expect(marked()).toEqual(["Image #1"]);
});

/** Pastes a clipboard with a small PNG in it into the note. */
async function pasteImage() {
  const clipboardData = new DataTransfer();
  clipboardData.items.add(new File([new Uint8Array([1])], "image.png", { type: "image/png" }));
  const before = note.value;
  note.focus();
  note.dispatchEvent(new ClipboardEvent("paste", { clipboardData, cancelable: true }));
  await vi.waitFor(() => expect(note.value).not.toBe(before));
}

/** Answers `save_image` as the Rust side would: `images/1.png`, then `images/2.png`, … */
function savedImages() {
  let n = 0;
  return () => `images/${++n}.png`;
}

test("images pasted into a note are let go of when it is cleared, unless saved", async () => {
  closePage();
  await open("DD/MM/YYYY", { save_image: savedImages() });
  const released = () => argsOf(calls, "trash_unsaved_images");

  await pasteImage();
  press(note, "Escape");
  await settle();
  expect(released()).toEqual([{ paths: ["images/1.png"], kept: [] }]);

  // Saved: the note (and the history) keeps one image, the other was taken out.
  type(note, "see ");
  await pasteImage();
  await pasteImage();
  expect(note.value).toBe("see [Image #1][Image #2]");
  note.setSelectionRange("see [Image #1]".length, note.value.length);
  document.execCommand("delete");
  press(note, "Enter");
  await settle();
  const saved = "see ![Image #1](images/2.png)";
  expect(argsOf(calls, "save_note")).toEqual([{ text: saved }]);
  expect(released()[1]).toEqual({ paths: ["images/2.png", "images/3.png"], kept: [saved] });
});

test("Ctrl+E hands the images of the note over to the Pad", async () => {
  closePage();
  await open("DD/MM/YYYY", { save_image: savedImages() });
  await pasteImage();
  await pasteImage();
  note.setSelectionRange(0, "[Image #1]".length);
  document.execCommand("delete");
  press(note, "e", { ctrlKey: true, code: "KeyE" });
  await settle();
  expect(argsOf(calls, "expand_to_pad")).toEqual([{ text: "![Image #2](images/2.png)" }]);
  expect(argsOf(calls, "trash_unsaved_images")).toEqual([{ paths: ["images/1.png"], kept: [] }]);
});

test("Ctrl+E keeps the note and shows why when the Pad draft can't be read", async () => {
  closePage();
  const error = "Failed to read pad-draft.md: stream did not contain valid UTF-8";
  await open("DD/MM/YYYY", {
    expand_to_pad: () => {
      throw error;
    },
  });
  type(note, "call mom");
  press(note, "e", { ctrlKey: true, code: "KeyE" });
  await settle();
  expect(note.value).toBe("call mom");
  expect(document.querySelector("#message")!.textContent).toBe(error);
  expect(calls.some((call) => call.cmd === "plugin:window|show")).toBe(true);
});
