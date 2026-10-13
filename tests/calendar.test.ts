import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { type DatePicker, datePicker } from "../src/calendar";
import { keydown, textField, type } from "./helpers";

let field: HTMLTextAreaElement;
let calendar: HTMLDivElement;
let picker: DatePicker;
const onError = vi.fn();

/** A field with `text` in it and the date picker on it, on 13 October 2026. */
function setUp(text: string, today = new Date(2026, 9, 13)) {
  vi.setSystemTime(today);
  field = textField(text);
  calendar = document.createElement("div");
  // Hidden until needed, as on the pages.
  calendar.hidden = true;
  document.body.append(calendar);
  picker = datePicker(field, calendar, { onError });
}

/** The month shown and the day picked in the calendar, as it writes them. */
const shown = () => [
  calendar.querySelector(".month")?.textContent,
  calendar.querySelector(".picked")?.textContent,
];

beforeEach(() => onError.mockClear());

afterEach(() => {
  vi.useRealTimers();
  document.body.replaceChildren();
});

describe("Enter on /date", () => {
  test("opens the calendar on today, leaving the text as it is", () => {
    setUp("call mom /date");
    expect(picker.resolve()).toBe(true);
    expect(calendar.hidden).toBe(false);
    expect(shown()).toEqual(["October 2026", "Tuesday, 13 October 2026"]);
    expect(calendar.querySelector(".selected")?.textContent).toBe("13");
    expect(calendar.querySelector(".distance")?.textContent).toBe("today");
    expect(field.value).toBe("call mom /date");
  });

  test("turns /date tomorrow into the date right away", () => {
    setUp("call mom /date tomorrow");
    expect(picker.resolve()).toBe(true);
    expect(calendar.hidden).toBe(true);
    expect(field.value).toBe("call mom 📅 14/10/2026 ");
  });

  test("turns /date and a date in the date format into the date", () => {
    setUp("/date 1/12/2026 call mom");
    picker.resolve();
    expect(field.value).toBe("📅 01/12/2026 call mom");
    // The space was already there: the caret goes past it.
    expect(field.selectionStart).toBe("📅 01/12/2026 ".length);
  });

  test("reports a word after /date that is not a date", () => {
    setUp("call mom /date someday");
    expect(picker.resolve()).toBe(true);
    expect(onError).toHaveBeenCalledWith(
      "Unknown date: someday — use DD/MM/YYYY or tomorrow",
    );
    expect(field.value).toBe("call mom /date someday");
  });

  test("does nothing without /date", () => {
    setUp("call mom /dates");
    expect(picker.resolve()).toBe(false);
    expect(calendar.hidden).toBe(true);
  });
});

describe("the calendar", () => {
  test("moves the day with the arrows", () => {
    setUp("/date");
    picker.resolve();
    expect(picker.keydown(keydown("ArrowRight"))).toBe(true);
    expect(shown()[1]).toBe("Wednesday, 14 October 2026");
    picker.keydown(keydown("ArrowDown"));
    expect(shown()[1]).toBe("Wednesday, 21 October 2026");
    picker.keydown(keydown("ArrowUp"));
    picker.keydown(keydown("ArrowLeft"));
    expect(shown()[1]).toBe("Tuesday, 13 October 2026");
  });

  test("crosses into the next month and year", () => {
    setUp("/date", new Date(2026, 11, 31));
    picker.resolve();
    picker.keydown(keydown("ArrowRight"));
    expect(shown()).toEqual(["January 2027", "Friday, 1 January 2027"]);
    picker.keydown(keydown("ArrowUp"));
    expect(shown()).toEqual(["December 2026", "Friday, 25 December 2026"]);
  });

  test("moves a month with PageUp and PageDown, keeping to the month's last day", () => {
    setUp("/date", new Date(2026, 0, 31));
    picker.resolve();
    picker.keydown(keydown("PageDown"));
    expect(shown()).toEqual(["February 2026", "Saturday, 28 February 2026"]);
    picker.keydown(keydown("PageUp"));
    picker.keydown(keydown("PageUp"));
    expect(shown()).toEqual(["December 2025", "Sunday, 28 December 2025"]);
  });

  test("Enter types the picked date over /date, with a space and the caret after it", () => {
    setUp("call mom /date");
    picker.resolve();
    picker.keydown(keydown("ArrowRight"));
    expect(picker.keydown(keydown("Enter"))).toBe(true);
    expect(calendar.hidden).toBe(true);
    expect(field.value).toBe("call mom 📅 14/10/2026 ");
    expect(field.selectionStart).toBe(field.value.length);
  });

  test("Esc closes only the calendar", () => {
    setUp("call mom /date");
    picker.resolve();
    const esc = keydown("Escape");
    // Handled, so the window does not close too.
    expect(picker.keydown(esc)).toBe(true);
    expect(esc.defaultPrevented).toBe(true);
    expect(calendar.hidden).toBe(true);
    expect(field.value).toBe("call mom /date");
  });

  test("closes when something is typed, which goes into the text", () => {
    setUp("call mom /date");
    picker.resolve();
    expect(picker.keydown(keydown("x"))).toBe(false);
    expect(calendar.hidden).toBe(true);
    picker.resolve();
    type(field, "x");
    expect(calendar.hidden).toBe(true);
    expect(field.value).toBe("call mom /datex");
  });

  test("stays open for a modifier alone, and leaves keys alone once closed", () => {
    setUp("/date");
    picker.resolve();
    expect(picker.keydown(keydown("Shift", { shiftKey: true }))).toBe(false);
    expect(calendar.hidden).toBe(false);
    picker.close();
    expect(picker.keydown(keydown("ArrowRight"))).toBe(false);
  });
});
