import { emit } from "@tauri-apps/api/event";
import { afterEach, expect, test, vi } from "vitest";
import html from "../src/windows/pad/index.html?raw";
import {
  type Call,
  argsOf,
  closePage,
  loadPage,
  mockTauri,
  onPlatform,
  press,
  settle,
  type,
} from "./helpers";

// The Pad, with its real page and code; only the Rust side is pretended, the draft file too.
let calls: Call[];
let text: HTMLTextAreaElement;
let draft: string;
/** The note the reader sends to edit next (Shift+Enter there). */
let edit: unknown;

/** Why the draft file can't be read, if it can't. */
let unreadable: string | null;

/** Opens an unpinned Pad with `saved` as its draft. */
async function open(saved = "") {
  draft = saved;
  edit = null;
  let n = 0;
  calls = mockTauri({
    get_appearance: { theme: "default", transparency: 0, scale: 100 },
    get_date_format: "DD/MM/YYYY",
    get_pad_pinned: false,
    get_pad_draft: () => {
      if (unreadable) throw unreadable;
      return draft;
    },
    set_pad_draft: (args: unknown) => {
      draft = (args as { text: string }).text;
    },
    take_pad_edit: () => {
      const taken = edit;
      edit = null;
      return taken;
    },
    save_image: () => `images/${++n}.png`,
  });
  await loadPage(html, () => import("../src/windows/pad/pad.ts"));
  text = document.querySelector("#text")!;
}

/** What the Pad let go of, in order. */
const released = () => argsOf(calls, "trash_unsaved_images");

/** Pastes a clipboard with a small PNG in it at the caret. */
async function pasteImage() {
  const clipboardData = new DataTransfer();
  clipboardData.items.add(new File([new Uint8Array([1])], "image.png", { type: "image/png" }));
  const before = text.value;
  text.focus();
  text.dispatchEvent(new ClipboardEvent("paste", { clipboardData, cancelable: true }));
  await vi.waitFor(() => expect(text.value).not.toBe(before));
}

afterEach(() => {
  closePage();
  unreadable = null;
});

test("a saved note lets go of the images pasted but left out of it", async () => {
  await open();
  await pasteImage();
  await pasteImage();
  expect(text.value).toBe("[Image #1][Image #2]");
  text.setSelectionRange(0, "[Image #1]".length);
  document.execCommand("delete");
  press(text, "Enter", { ctrlKey: true });
  await vi.waitFor(() => expect(released()).toHaveLength(1));
  expect(argsOf(calls, "save_note")).toEqual([{ text: "![Image #2](images/2.png)" }]);
  // Both are sent; the Rust side keeps the one the saved note links to.
  expect(released()).toEqual([{ paths: ["images/1.png", "images/2.png"], kept: [] }]);
  expect(draft).toBe("");
});

test("the draft's images are held; an edit lets go of the images pasted into it", async () => {
  await open("old ![Image #1](images/drafted.png)");
  expect(text.value).toBe("old [Image #1]");
  expect(released()).toEqual([]);

  // The draft is put aside for a note from the reader; the draft file still links its image.
  edit = { name: "todo.md", note: { line: 2, time: "09:00", done: false, text: "call" } };
  await emit("pad-edit");
  await vi.waitFor(() => expect(text.value).toBe("call"));
  expect(released()).toEqual([{ paths: ["images/drafted.png"], kept: [] }]);

  // An image pasted into the edit, then the edit cancelled: the draft comes back.
  text.setSelectionRange(text.value.length, text.value.length);
  await pasteImage();
  press(text, "Escape");
  await vi.waitFor(() => expect(text.value).toBe("old [Image #1]"));
  expect(released()[1]).toEqual({ paths: ["images/1.png"], kept: [] });

  // The draft's image is held again: saving the draft without it lets go of it.
  text.select();
  type(text, "new");
  press(text, "Enter", { ctrlKey: true });
  await vi.waitFor(() => expect(released()).toHaveLength(3));
  expect(argsOf(calls, "save_note")).toEqual([{ text: "new" }]);
  expect(released()[2]).toEqual({ paths: ["images/drafted.png"], kept: [] });
});

test("text moved here from the input window brings its images with it", async () => {
  await open("draft");
  draft = "draft\n![Image #1](images/moved.png)";
  await emit("pad-draft-changed");
  await vi.waitFor(() => expect(text.value).toBe("draft\n[Image #1]"));
  text.select();
  type(text, "kept the text only");
  press(text, "Enter", { ctrlKey: true });
  await vi.waitFor(() => expect(released()).toHaveLength(1));
  expect(released()).toEqual([{ paths: ["images/moved.png"], kept: [] }]);
  await settle();
});

test("a draft that can't be read is shown as an error and never saved over", async () => {
  unreadable = "Failed to read pad-draft.md: stream did not contain valid UTF-8";
  await open("the draft on disk");
  const status = document.querySelector("#status")!;
  expect(text.value).toBe("");
  expect(status.textContent).toBe(unreadable);
  expect(calls.some((call) => call.cmd === "reveal_window")).toBe(true);

  // Neither typing (saved 300 ms after it stops) nor closing saves over it.
  type(text, "typed meanwhile");
  await new Promise((resolve) => setTimeout(resolve, 350));
  expect(status.textContent).toBe(unreadable);
  press(text, "Escape");
  await settle();
  expect(argsOf(calls, "set_pad_draft")).toEqual([]);
  expect(draft).toBe("the draft on disk");
});

test("tags are highlighted whole, in any script, as the notes files understand them", async () => {
  await open();
  type(text, "#हिंदी note #work/quicklly-2/ #1 C# a#b #Ünï_x");
  const tags = Array.from(document.querySelectorAll("#backdrop .tag"), (tag) => tag.textContent);
  expect(tags).toEqual(["#हिंदी", "#work/quicklly-2", "#Ünï_x"]);
  await settle();
});

test("the Enter that ends an input method's composition is left to it", async () => {
  await open();
  type(text, "- とうきょう");
  for (const composing of [{ isComposing: true }, { keyCode: 229 }]) {
    // Neither a new list item nor a saved note.
    expect(press(text, "Enter", composing).defaultPrevented).toBe(false);
    expect(press(text, "Enter", { ctrlKey: true, ...composing }).defaultPrevented).toBe(false);
  }
  await settle();
  expect(text.value).toBe("- とうきょう");
  expect(argsOf(calls, "save_note")).toEqual([]);
});

test("the hints name the keys as the system does", async () => {
  const help = () => document.querySelector("#help")!.textContent;
  /** The help of an empty Pad, the pin's tooltip and the help once there is text. */
  const hints = () => {
    const shown = [help(), document.querySelector<HTMLElement>("#pin")!.title];
    type(text, "call mom");
    return [...shown, help()];
  };
  await open();
  expect(hints()).toEqual([
    "← quick line · Ctrl+P pin · Esc close",
    "Ctrl+P",
    "Ctrl+Enter save · Ctrl+P pin · Esc close",
  ]);
  closePage();
  onPlatform("darwin");
  await open();
  expect(hints()).toEqual([
    "← quick line · ⌘P pin · Esc close",
    "⌘P",
    "⌘Enter save · ⌘P pin · Esc close",
  ]);
});

test("on macOS ⌘Enter saves and ⌘P pins, not Ctrl", async () => {
  onPlatform("darwin");
  await open();
  type(text, "call mom");
  press(text, "Enter", { ctrlKey: true });
  press(text, "p", { ctrlKey: true, code: "KeyP" });
  await settle();
  expect(argsOf(calls, "save_note")).toEqual([]);
  expect(argsOf(calls, "set_pad_pinned")).toEqual([]);
  press(text, "Enter", { metaKey: true });
  press(text, "p", { metaKey: true, code: "KeyP" });
  await settle();
  expect(argsOf(calls, "save_note")).toEqual([{ text: "call mom" }]);
  expect(argsOf(calls, "set_pad_pinned")).toEqual([{ pinned: true }]);
});
