import { afterEach, expect, test, vi } from "vitest";
import { blockCaret } from "../src/shared/blockCaret";
import { textField } from "./helpers";

const NBSP = String.fromCharCode(0xa0);

let field: HTMLTextAreaElement;
let caret: HTMLDivElement;
let mirror: HTMLDivElement;

/**
 * A field with `value` and the caret at `at`, its block caret drawn. The page has no layout, so
 * the mirror is 100px wide with 16px of padding and a 2px border on the right (its text ends at
 * 82px), and the marked character is laid out at `marker`.
 */
function setUp(value: string, at: number, marker = new DOMRect(40, 0, 8, 16)) {
  field = textField(value);
  field.style.lineHeight = "20px";
  caret = document.createElement("div");
  mirror = document.createElement("div");
  mirror.style.paddingRight = "16px";
  mirror.style.borderRightWidth = "2px";
  document.body.append(caret, mirror);
  vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (
    this: Element,
  ) {
    if (this === mirror) return new DOMRect(0, 0, 100, 40);
    return this.parentElement === mirror ? marker : new DOMRect();
  });
  field.setSelectionRange(at, at);
  blockCaret(field, caret, mirror)();
}

/** The character the caret is drawn over in the mirror. */
const marked = () => mirror.querySelector("span")!.textContent;

afterEach(() => {
  vi.restoreAllMocks();
  document.body.replaceChildren();
});

test("lays out the whole text, so the mirror wraps like the field", () => {
  setUp("one two\nthree", 2);
  expect(mirror.textContent).toBe("one two\nthree");
  expect(marked()).toBe("e");
  expect(caret.textContent).toBe("e");
  expect(caret.style.left).toBe("40px");
});

test("at the end of a line, goes just after its last character", () => {
  setUp("one\ntwo", 3);
  // Nothing is added to the text, so the last word can't wrap differently.
  expect(mirror.textContent).toBe("one\ntwo");
  expect(marked()).toBe("e");
  expect(caret.textContent).toBe(NBSP);
  expect(caret.style.left).toBe("48px");
});

test("at the end of a full line, stays at the edge of the text", () => {
  setUp("hanging spaces   ", 17, new DOMRect(90, 0, 8, 16));
  expect(caret.style.left).toBe("82px");
});

test("on an empty line, covers a space", () => {
  setUp("a\n\nb", 2);
  expect(mirror.textContent).toBe(`a\n${NBSP}\nb`);
  expect(caret.textContent).toBe(NBSP);
});

test("after a selection, is placed in its container, not in the page", () => {
  setUp("text", 4);
  field.setSelectionRange(0, 4);
  blockCaret(field, caret, mirror)();
  // Like a browser: a hidden element has no offset parent.
  const container = document.createElement("div");
  Object.defineProperty(caret, "offsetParent", { get: () => (caret.hidden ? null : container) });
  vi.mocked(Element.prototype.getBoundingClientRect).mockImplementation(function (this: Element) {
    if (this === mirror) return new DOMRect(0, 0, 100, 40);
    if (this === container) return new DOMRect(30, 10, 100, 40);
    return this.parentElement === mirror ? new DOMRect(40, 0, 8, 16) : new DOMRect();
  });
  field.setSelectionRange(4, 4);
  blockCaret(field, caret, mirror)();
  expect(caret.hidden).toBe(false);
  expect(caret.style.left).toBe("18px");
});

test("is hidden while text is selected", () => {
  setUp("text", 4);
  field.setSelectionRange(0, 4);
  blockCaret(field, caret, mirror)();
  expect(caret.hidden).toBe(true);
});
