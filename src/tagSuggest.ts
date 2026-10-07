import { invoke } from "@tauri-apps/api/core";

/**
 * A `#tag` being typed right before the caret: `#` at the start of a word, then nothing yet or
 * a letter and more tag characters, like the notes files understand tags.
 */
const PARTIAL_TAG = /(?:^|\s)#((?:\p{L}[\p{L}\p{N}_-]*)?)$/u;
/** Tag characters right after the caret: picking a tag in the middle of one replaces them too. */
const TAG_REST = /^[\p{L}\p{N}_-]*/u;

/** The partial tag before the caret: where its `#` is and what follows the `#`. */
interface TypedTag {
  start: number;
  query: string;
}

export interface TagSuggest {
  /** Fetches the tags again, so tag files made since show up; call when the window is shown. */
  load(): Promise<void>;
  /** Checks the text before the caret again; call after the text is changed by code. */
  update(): void;
  /** Handles ↑ ↓ Enter Tab Esc while the list is shown; returns true when the key was used. */
  keydown(e: KeyboardEvent): boolean;
}

/**
 * Suggests the existing tags in `list` while a `#tag` is typed in `input`, and types the picked
 * one. The same in every window, so tags are completed the same way everywhere. `onChange`
 * runs after the list is shown, changed or hidden, to fit the window to it.
 */
export function tagSuggest(
  input: HTMLTextAreaElement,
  list: HTMLUListElement,
  {
    enabled = () => true,
    onChange = () => {},
  }: { enabled?: () => boolean; onChange?: () => void } = {},
): TagSuggest {
  let tags: string[] = [];
  let partial: TypedTag | null = null;
  let matches: string[] = [];
  let selected = 0;
  /** The partial tag whose list was closed with Esc; it stays closed until that changes. */
  let closed: TypedTag | null = null;

  const same = (a: TypedTag | null, b: TypedTag | null) =>
    a?.start === b?.start && a?.query === b?.query;

  function find(): TypedTag | null {
    const { selectionStart, selectionEnd, value } = input;
    if (!enabled() || selectionStart !== selectionEnd) return null;
    const match = PARTIAL_TAG.exec(value.slice(0, selectionStart));
    if (!match) return null;
    return { start: selectionStart - match[1].length - 1, query: match[1] };
  }

  function filter(query: string) {
    const typed = query.toLowerCase();
    const found = tags.filter((tag) => tag.toLowerCase().startsWith(typed));
    // Nothing to complete when the one suggestion is the tag already typed, so Enter saves.
    return found.length === 1 && found[0].toLowerCase() === typed ? [] : found;
  }

  function update() {
    const found = find();
    if (!same(found, closed)) closed = null;
    const next = found && !closed ? filter(found.query) : [];
    const unchanged =
      same(found, partial) &&
      next.length === matches.length &&
      next.every((tag, i) => tag === matches[i]);
    if (unchanged) return;
    partial = found;
    matches = next;
    selected = 0;
    render();
  }

  function render() {
    list.hidden = matches.length === 0;
    const typed = partial?.query.length ?? 0;
    list.replaceChildren(
      ...matches.map((tag, i) => {
        const li = document.createElement("li");
        li.classList.toggle("selected", i === selected);
        const pointer = document.createElement("span");
        pointer.textContent = i === selected ? "❯" : "";
        const name = document.createElement("span");
        const mark = document.createElement("mark");
        mark.textContent = `#${tag.slice(0, typed)}`;
        name.append(mark, tag.slice(typed));
        li.append(pointer, name);
        // mousedown, so the text keeps the focus.
        li.addEventListener("mousedown", (e) => {
          e.preventDefault();
          pick(tag);
        });
        return li;
      }),
    );
    list.children[selected]?.scrollIntoView({ block: "nearest" });
    onChange();
  }

  /** Types `#tag ` over the partial tag, keeping Ctrl+Z working; the list then closes. */
  function pick(tag: string) {
    if (!partial) return;
    const caret = input.selectionStart;
    const end = caret + TAG_REST.exec(input.value.slice(caret))![0].length;
    input.setSelectionRange(partial.start, end);
    document.execCommand("insertText", false, `#${tag} `);
    update();
  }

  function keydown(e: KeyboardEvent) {
    if (matches.length === 0 || e.shiftKey || e.ctrlKey || e.altKey || e.metaKey) return false;
    if (e.key === "ArrowUp" || e.key === "ArrowDown") {
      const step = e.key === "ArrowUp" ? -1 : 1;
      selected = (selected + step + matches.length) % matches.length;
      render();
    } else if (e.key === "Enter" || e.key === "Tab") {
      pick(matches[selected]);
    } else if (e.key === "Escape") {
      closed = partial;
      matches = [];
      render();
    } else {
      return false;
    }
    e.preventDefault();
    return true;
  }

  async function load() {
    try {
      tags = await invoke<string[]>("list_tags");
    } catch {
      // Suggestions are only a help: keep the tags already known.
    }
    update();
  }

  input.addEventListener("input", update);
  document.addEventListener("selectionchange", update);
  return { load, update, keydown };
}
