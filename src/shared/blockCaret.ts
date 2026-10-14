const NBSP = String.fromCharCode(0xa0);

/**
 * Terminal-style block caret for a textarea whose native caret is hidden. `caret` is drawn over
 * the character at the cursor, which is found by laying out the text before the cursor on
 * `mirror`: an invisible copy of the textarea with the same font, width and padding, in the
 * same positioned container as `caret`.
 *
 * Follows focus and selection changes by itself; call the returned function after the text
 * changes or scrolls.
 */
export function blockCaret(
  input: HTMLTextAreaElement,
  caret: HTMLElement,
  mirror: HTMLElement,
): () => void {
  function update() {
    const { selectionStart, selectionEnd, value } = input;
    // A selection is shown by its highlight instead.
    if (document.activeElement !== input || selectionStart !== selectionEnd) {
      caret.hidden = true;
      return;
    }
    // Over an empty field the caret covers the first letter of the placeholder, like a terminal.
    const under = value === "" ? input.placeholder[0] : value[selectionStart];
    const marker = document.createElement("span");
    marker.textContent = under && under !== "\n" ? under : NBSP;
    mirror.replaceChildren(value.slice(0, selectionStart), marker);

    const lineHeight = parseFloat(getComputedStyle(input).lineHeight);
    const rect = marker.getBoundingClientRect();
    const box = (caret.offsetParent ?? document.body).getBoundingClientRect();
    // The marker box is the glyph height; centre the caret on the whole line instead.
    const lineTop = rect.top - (lineHeight - rect.height) / 2;
    caret.textContent = marker.textContent;
    caret.style.left = `${rect.left - box.left}px`;
    caret.style.top = `${lineTop - box.top - input.scrollTop}px`;
    caret.style.width = `${rect.width}px`;
    caret.style.height = `${lineHeight}px`;
    caret.style.lineHeight = `${lineHeight}px`;
    caret.hidden = false;
  }

  document.addEventListener("selectionchange", update);
  input.addEventListener("focus", update);
  input.addEventListener("blur", update);
  return update;
}
