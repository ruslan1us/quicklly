import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, test, vi } from "vitest";
import {
  type Images,
  fromMarkdown,
  imagePaths,
  nextImage,
  pasteImages,
  releaseImages,
  toMarkdown,
} from "../src/shared/images";
import { type Call, argsOf, mockTauri, settle, textField } from "./helpers";

const images = new Map([
  [1, "images/2026-10-07_14-32-05.png"],
  [2, "images/2026-10-07_14-32-05_2.jpg"],
]);
const text = "see [Image #1] and\n  [Image #2], not [Image #3] #todo";
const markdown =
  "see ![Image #1](images/2026-10-07_14-32-05.png) and\n" +
  "  ![Image #2](images/2026-10-07_14-32-05_2.jpg), not [Image #3] #todo";

describe("toMarkdown and fromMarkdown", () => {
  test("turn the labels of known images into Markdown; other labels stay", () => {
    expect(toMarkdown(text, images)).toBe(markdown);
    expect(toMarkdown("plain [Image #1]", new Map())).toBe("plain [Image #1]");
  });

  test("turn Markdown back into labels and paths", () => {
    expect(fromMarkdown(markdown)).toEqual({ text, images });
  });

  test("round trip", () => {
    const shown = fromMarkdown(markdown);
    expect(toMarkdown(shown.text, shown.images)).toBe(markdown);
  });

  test("keep the numbers of a note's images, whatever they start from", () => {
    const note = "#quicklly ![Image #2](images/a.png) then ![Image #1](../images/b.png)";
    const shown = fromMarkdown(note);
    expect(shown).toEqual({
      text: "#quicklly [Image #2] then [Image #1]",
      images: new Map([
        [2, "images/a.png"],
        [1, "../images/b.png"],
      ]),
    });
    expect(toMarkdown(shown.text, shown.images)).toBe(note);
  });

  test("give clashing numbers of different images numbers of their own", () => {
    // A Pad draft with an image, then a note from the input window with its own Image #1.
    const merged =
      "![Image #1](images/a.png) draft\n" +
      "![Image #1](images/b.png) moved ![Image #2](images/c.png) ![Image #1](images/a.png)";
    expect(fromMarkdown(merged)).toEqual({
      text: "[Image #1] draft\n[Image #2] moved [Image #3] [Image #1]",
      images: new Map([
        [1, "images/a.png"],
        [2, "images/b.png"],
        [3, "images/c.png"],
      ]),
    });
  });
});

describe("nextImage", () => {
  test("is one more than the highest number so far", () => {
    expect(nextImage(new Map())).toBe(1);
    expect(nextImage(images)).toBe(3);
    expect(nextImage(new Map([[5, "images/x.png"]]))).toBe(6);
  });
});

describe("imagePaths", () => {
  test("lists the paths of a note's images in order, each once", () => {
    const note = "![Image #2](images/b.png) ![Image #1](images/a.png) ![Image #3](images/b.png)";
    expect(imagePaths(note)).toEqual(["images/b.png", "images/a.png"]);
  });

  test("ignores labels without a path", () => {
    expect(imagePaths("just [Image #1] and text")).toEqual([]);
  });
});

describe("pasteImages", () => {
  const path = "images/2026-10-13_15-40-00.png";
  let calls: Call[];
  let pasted: Images;
  let held: Set<string>;
  const onError = vi.fn();

  /** A field with "see " in it that images paste into, saved as `saveImage` answers. */
  function setUp(saveImage: unknown = path) {
    calls = mockTauri({ save_image: saveImage });
    pasted = new Map();
    held = new Set();
    onError.mockClear();
    const field = textField("see ");
    pasteImages(field, { images: () => pasted, held, onError });
    return field;
  }

  /** Pastes a clipboard with a small PNG in it, and `text` if given. */
  function paste(field: HTMLTextAreaElement, text?: string) {
    const clipboardData = new DataTransfer();
    const image = new File([new Uint8Array([1, 2, 3])], "image.png", { type: "image/png" });
    clipboardData.items.add(image);
    if (text !== undefined) clipboardData.setData("text/plain", text);
    const event = new ClipboardEvent("paste", { clipboardData, cancelable: true });
    field.dispatchEvent(event);
    return event;
  }

  afterEach(() => {
    clearMocks();
    document.body.replaceChildren();
  });

  test("saves a pasted image and types its label", async () => {
    const field = setUp();
    expect(paste(field).defaultPrevented).toBe(true);
    await vi.waitFor(() => expect(field.value).toBe("see [Image #1]"));
    expect(argsOf(calls, "save_image")).toEqual([new Uint8Array([1, 2, 3])]);
    expect(pasted).toEqual(new Map([[1, path]]));
    expect(held).toEqual(new Set([path]));
  });

  test("numbers a next image after the ones already pasted", async () => {
    const field = setUp();
    pasted.set(1, "images/a.png");
    paste(field);
    await vi.waitFor(() => expect(field.value).toBe("see [Image #2]"));
  });

  test("pastes as usual when the clipboard has text too", async () => {
    const field = setUp();
    expect(paste(field, "A1\tB1").defaultPrevented).toBe(false);
    await settle();
    expect(argsOf(calls, "save_image")).toEqual([]);
    expect(field.value).toBe("see ");
  });

  test("reports an image that could not be saved", async () => {
    const field = setUp(() => {
      throw "The notes folder is gone";
    });
    paste(field);
    await vi.waitFor(() => expect(onError).toHaveBeenCalledWith("The notes folder is gone"));
    expect(field.value).toBe("see ");
    expect(held.size).toBe(0);
  });
});

describe("releaseImages", () => {
  afterEach(() => clearMocks());

  test("sends the held images with the texts to keep, and holds none after", () => {
    const calls = mockTauri();
    const held = new Set(["images/a.png", "images/b.png"]);
    releaseImages(held, ["![Image #1](images/a.png)"]);
    expect(argsOf(calls, "trash_unsaved_images")).toEqual([
      { paths: ["images/a.png", "images/b.png"], kept: ["![Image #1](images/a.png)"] },
    ]);
    expect(held.size).toBe(0);
  });

  test("sends nothing when no image is held", () => {
    const calls = mockTauri();
    releaseImages(new Set());
    expect(calls).toEqual([]);
  });
});
