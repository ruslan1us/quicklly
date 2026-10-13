import { afterEach, describe, expect, test } from "vitest";
import {
  type DateFormat,
  commandDate,
  deleteDatesWhole,
  findDateCommand,
  formatDate,
  parseDate,
  splitDoneDate,
  toDisplay,
  toStored,
  tomorrow,
} from "../src/dates";
import { press, textField } from "./helpers";

/** A date as `YYYY-MM-DD`, to compare dates by their day. */
const day = (date: Date | null) => (date ? formatDate(date, "YYYY-MM-DD") : null);

/** 15 October 2026 in each format. */
const FORMATS: [DateFormat, string][] = [
  ["DD/MM/YYYY", "15/10/2026"],
  ["MM/DD/YYYY", "10/15/2026"],
  ["YYYY-MM-DD", "2026-10-15"],
  ["DD.MM.YYYY", "15.10.2026"],
];

describe("formatDate and parseDate", () => {
  test.each(FORMATS)("%s", (format, shown) => {
    expect(formatDate(new Date(2026, 9, 15), format)).toBe(shown);
    expect(day(parseDate(shown, format))).toBe("2026-10-15");
  });

  test("pads one-digit days and months, and accepts them typed without", () => {
    expect(formatDate(new Date(2027, 0, 5), "DD/MM/YYYY")).toBe("05/01/2027");
    expect(day(parseDate("5/1/2027", "DD/MM/YYYY"))).toBe("2027-01-05");
  });

  test("accepts a leap day only in a leap year", () => {
    expect(day(parseDate("29/02/2028", "DD/MM/YYYY"))).toBe("2028-02-29");
    expect(parseDate("29/02/2026", "DD/MM/YYYY")).toBeNull();
  });

  test.each([
    ["31/02/2026", "DD/MM/YYYY"],
    ["32/01/2026", "DD/MM/YYYY"],
    ["00/10/2026", "DD/MM/YYYY"],
    ["15/13/2026", "DD/MM/YYYY"],
    ["15/10/26", "DD/MM/YYYY"],
    ["15-10-2026", "DD/MM/YYYY"],
    ["", "DD/MM/YYYY"],
    ["02/31/2026", "MM/DD/YYYY"],
    ["2026-02-30", "YYYY-MM-DD"],
    ["31.04.2026", "DD.MM.YYYY"],
    ["15/10/2026", "DD.MM.YYYY"],
  ] as [string, DateFormat][])("rejects %s in %s", (text, format) => {
    expect(parseDate(text, format)).toBeNull();
  });
});

describe("tomorrow", () => {
  test("crosses the end of a month and a year", () => {
    expect(day(tomorrow(new Date(2026, 9, 31)))).toBe("2026-11-01");
    expect(day(tomorrow(new Date(2026, 11, 31, 23, 59)))).toBe("2027-01-01");
  });

  test("knows leap years", () => {
    expect(day(tomorrow(new Date(2028, 1, 28)))).toBe("2028-02-29");
    expect(day(tomorrow(new Date(2026, 1, 28)))).toBe("2026-03-01");
  });
});

describe("toStored and toDisplay", () => {
  const stored = "call mom 📅 2026-10-15 and\n  📅 2027-01-05 #todo ✅ 2026-10-13";

  test.each(FORMATS)("round trip in %s", (format, shown) => {
    const display = toDisplay(stored, format);
    expect(display).toContain(`📅 ${shown}`);
    expect(toStored(display, format)).toBe(stored);
  });

  test("shows each date in the format, keeping its mark", () => {
    expect(toDisplay(stored, "DD/MM/YYYY")).toBe(
      "call mom 📅 15/10/2026 and\n  📅 05/01/2027 #todo ✅ 13/10/2026",
    );
    expect(toDisplay(stored, "MM/DD/YYYY")).toBe(
      "call mom 📅 10/15/2026 and\n  📅 01/05/2027 #todo ✅ 10/13/2026",
    );
  });

  test("stores a date typed with one-digit day and month", () => {
    expect(toStored("x 📅 5/1/2027", "DD/MM/YYYY")).toBe("x 📅 2027-01-05");
  });

  test("a change of format keeps the dates", () => {
    const before = toDisplay(stored, "DD/MM/YYYY");
    const after = toDisplay(toStored(before, "DD/MM/YYYY"), "DD.MM.YYYY");
    expect(after).toBe(toDisplay(stored, "DD.MM.YYYY"));
  });

  test.each(["", "plain note", "15/10/2026 and 2026-10-15", "📅 tomorrow", "📅2026-10-15"])(
    "leaves a text without dates unchanged: %j",
    (text) => {
      for (const [format] of FORMATS) {
        expect(toDisplay(text, format)).toBe(text);
        expect(toStored(text, format)).toBe(text);
      }
    },
  );

  test("leaves invalid dates and dates inside longer numbers as typed", () => {
    expect(toStored("📅 31/02/2026", "DD/MM/YYYY")).toBe("📅 31/02/2026");
    expect(toDisplay("📅 2026-02-31", "DD/MM/YYYY")).toBe("📅 2026-02-31");
    expect(toDisplay("📅 2026-10-151", "DD/MM/YYYY")).toBe("📅 2026-10-151");
    expect(toStored("📅 15/10/20261", "DD/MM/YYYY")).toBe("📅 15/10/20261");
  });

  test("leaves dates in links and image paths alone", () => {
    const text =
      "https://example.com/2026-10-15/a?d=15/10/2026 ![Image #1](images/2026-10-15_14-32-05.png)";
    expect(toDisplay(text, "DD/MM/YYYY")).toBe(text);
    expect(toStored(text, "DD/MM/YYYY")).toBe(text);
  });
});

describe("findDateCommand", () => {
  test("finds /date as a word of its own, with the word after it", () => {
    expect(findDateCommand("/date")).toEqual({ start: 0, end: 5, arg: undefined });
    expect(findDateCommand("call mom /date")).toEqual({ start: 9, end: 14, arg: undefined });
    expect(findDateCommand("call /date tomorrow please")).toEqual({
      start: 5,
      end: 19,
      arg: "tomorrow",
    });
    expect(findDateCommand("/date  15/10/2026")).toEqual({ start: 0, end: 17, arg: "15/10/2026" });
  });

  test("takes the word after it only from the same line", () => {
    expect(findDateCommand("a\n/date\nnext")).toEqual({ start: 2, end: 7, arg: undefined });
    expect(findDateCommand("/date \ntomorrow")).toEqual({ start: 0, end: 5, arg: undefined });
  });

  test("finds the first one", () => {
    expect(findDateCommand("/date a /date")?.start).toBe(0);
  });

  test.each(["/dates", "x/date", "/date1", "no command", ""])("finds none in %j", (text) => {
    expect(findDateCommand(text)).toBeNull();
  });
});

describe("commandDate", () => {
  const today = new Date(2026, 11, 31);

  test("tomorrow, in any case", () => {
    expect(day(commandDate("tomorrow", today, "DD/MM/YYYY"))).toBe("2027-01-01");
    expect(day(commandDate("TOMORROW", today, "DD/MM/YYYY"))).toBe("2027-01-01");
  });

  test("a date in the format", () => {
    expect(day(commandDate("15/10/2026", today, "DD/MM/YYYY"))).toBe("2026-10-15");
    expect(day(commandDate("10/15/2026", today, "MM/DD/YYYY"))).toBe("2026-10-15");
  });

  test("nothing else", () => {
    expect(commandDate("someday", today, "DD/MM/YYYY")).toBeNull();
    expect(commandDate("31/02/2026", today, "DD/MM/YYYY")).toBeNull();
  });
});

describe("splitDoneDate", () => {
  test("takes the done date off the end of the note", () => {
    const { text, done } = splitDoneDate("call mom ✅ 2026-10-13");
    expect(text).toBe("call mom");
    expect(day(done)).toBe("2026-10-13");
  });

  test("looks only at the end of the first line of a longer note", () => {
    const { text, done } = splitDoneDate("call mom ✅ 2026-10-13\n  and dad\n\n  later");
    expect(text).toBe("call mom\n  and dad\n\n  later");
    expect(day(done)).toBe("2026-10-13");
    expect(splitDoneDate("call mom\n  and dad ✅ 2026-10-13")).toEqual({
      text: "call mom\n  and dad ✅ 2026-10-13",
      done: null,
    });
  });

  test("leaves a done date in the middle of the line", () => {
    const text = "✅ 2026-10-13 was the day";
    expect(splitDoneDate(text)).toEqual({ text, done: null });
  });

  test("leaves a note without a done date, or with an invalid one", () => {
    expect(splitDoneDate("call mom 📅 2026-10-15")).toEqual({
      text: "call mom 📅 2026-10-15",
      done: null,
    });
    expect(splitDoneDate("x ✅ 2026-02-31")).toEqual({ text: "x ✅ 2026-02-31", done: null });
  });
});

describe("deleteDatesWhole", () => {
  const text = "call mom 📅 15/10/2026 now";
  const start = "call mom ".length;
  const end = "call mom 📅 15/10/2026".length;

  /** A field with `text` in it, the caret at `caret`, and dates going as a whole. */
  function setUp(caret: number, value = text) {
    const field = textField(value);
    field.setSelectionRange(caret, caret);
    deleteDatesWhole(field);
    return field;
  }

  afterEach(() => document.body.replaceChildren());

  test("Backspace right after a date removes all of it", () => {
    const field = setUp(end);
    expect(press(field, "Backspace").defaultPrevented).toBe(true);
    expect(field.value).toBe("call mom  now");
    expect(field.selectionStart).toBe(start);
  });

  test("Delete right before a date removes all of it", () => {
    const field = setUp(start);
    expect(press(field, "Delete").defaultPrevented).toBe(true);
    expect(field.value).toBe("call mom  now");
  });

  test("a done date goes as a whole too", () => {
    const field = setUp("x ✅ 13/10/2026".length, "x ✅ 13/10/2026");
    press(field, "Backspace");
    expect(field.value).toBe("x ");
  });

  test.each([
    ["Backspace inside a date", "Backspace", end - 1, {}],
    ["Delete after a date", "Delete", end, {}],
    ["Backspace before a date", "Backspace", start, {}],
    ["Ctrl+Backspace", "Backspace", end, { ctrlKey: true }],
  ])("%s deletes as usual", (_, key, caret, init) => {
    const field = setUp(caret);
    expect(press(field, key, init).defaultPrevented).toBe(false);
    expect(field.value).toBe(text);
  });

  test("a selection deletes as usual", () => {
    const field = setUp(end);
    field.setSelectionRange(end - 4, end);
    expect(press(field, "Backspace").defaultPrevented).toBe(false);
  });

  test("an invalid date is plain text", () => {
    const field = setUp("📅 31/02/2026".length, "📅 31/02/2026");
    expect(press(field, "Backspace").defaultPrevented).toBe(false);
  });
});
