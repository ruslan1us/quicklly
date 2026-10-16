import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { type TagSuggest, tagSuggest } from "../src/shared/tagSuggest";
import { keydown, mockTauri, textField, type } from "./helpers";

let field: HTMLTextAreaElement;
let list: HTMLUListElement;
let suggestions: TagSuggest;

/** A field with tag suggestions from the existing tags `tags`. */
async function setUp(tags = ["todo", "today", "work"]) {
  mockTauri({ list_tags: tags });
  field = textField();
  list = document.createElement("ul");
  list.hidden = true;
  document.body.append(list);
  suggestions = tagSuggest(field, list);
  await suggestions.load();
}

/** The tags in the list, as it shows them: the selected one after a `❯`. */
const shown = () => Array.from(list.children, (li) => li.textContent);

afterEach(() => {
  clearMocks();
  document.body.replaceChildren();
});

test("lists the tags that start with what is typed after #", async () => {
  await setUp();
  type(field, "call mom #to");
  expect(list.hidden).toBe(false);
  expect(shown()).toEqual(["❯#todo", "#today"]);
  type(field, "da");
  expect(shown()).toEqual(["❯#today"]);
});

test("lists all tags for a lone #, and none in the middle of a word", async () => {
  await setUp();
  type(field, "#");
  expect(shown()).toHaveLength(3);
  type(field, " a#t");
  expect(list.hidden).toBe(true);
});

test("Enter types the selected tag over the typed one, with a space", async () => {
  await setUp();
  type(field, "call mom #to");
  suggestions.keydown(keydown("ArrowDown"));
  expect(shown()).toEqual(["#todo", "❯#today"]);
  expect(suggestions.keydown(keydown("Enter"))).toBe(true);
  expect(field.value).toBe("call mom #today ");
  expect(field.selectionStart).toBe(field.value.length);
  expect(list.hidden).toBe(true);
});

test("Tab picks too, and ↑ goes round to the last tag", async () => {
  await setUp();
  type(field, "#to");
  suggestions.keydown(keydown("ArrowUp"));
  expect(suggestions.keydown(keydown("Tab"))).toBe(true);
  expect(field.value).toBe("#today ");
});

test("Esc closes the list until the tag typed changes", async () => {
  await setUp();
  type(field, "#to");
  expect(suggestions.keydown(keydown("Escape"))).toBe(true);
  expect(list.hidden).toBe(true);
  // Closed, so Enter is left to the window, to save the note.
  expect(suggestions.keydown(keydown("Enter"))).toBe(false);
  type(field, "d");
  expect(shown()).toEqual(["❯#todo", "#today"]);
});

test("offers nothing when the tag is already typed in full", async () => {
  await setUp();
  type(field, "#work");
  expect(list.hidden).toBe(true);
});

test("suggests nested tags and types the whole tag", async () => {
  await setUp(["todo", "work", "work/quicklly"]);
  type(field, "#wo");
  expect(shown()).toEqual(["❯#work", "#work/quicklly"]);
  type(field, "rk/");
  expect(shown()).toEqual(["❯#work/quicklly"]);
  type(field, "q");
  expect(shown()).toEqual(["❯#work/quicklly"]);
  expect(suggestions.keydown(keydown("Enter"))).toBe(true);
  expect(field.value).toBe("#work/quicklly ");
});

test("picking in the middle of a nested tag replaces all of it", async () => {
  await setUp(["work/quicklly"]);
  type(field, "#work/qx");
  field.setSelectionRange(3, 3);
  field.dispatchEvent(new Event("input"));
  expect(shown()).toEqual(["❯#work/quicklly"]);
  suggestions.keydown(keydown("Enter"));
  expect(field.value).toBe("#work/quicklly ");
});

test("tags with the vowel signs of their script are suggested and replaced whole", async () => {
  await setUp(["हिंदी", "हिंदू", "work"]);
  type(field, "नोट #हिं");
  expect(shown()).toEqual(["❯#हिंदी", "#हिंदू"]);
  type(field, "दx");
  field.setSelectionRange("नोट #हि".length, "नोट #हि".length);
  field.dispatchEvent(new Event("input"));
  expect(shown()).toEqual(["❯#हिंदी", "#हिंदू"]);
  suggestions.keydown(keydown("Enter"));
  expect(field.value).toBe("नोट #हिंदी ");
});
