const NBSP = String.fromCharCode(0xa0);

/**
 * Terminal-style block caret for a textarea whose native caret is hidden. `caret` is drawn over
 * the character at the cursor, which is found by laying out the textarea's text on `mirror`:
 * an invisible copy of the textarea with the same font, width and padding, in the same
 * positioned container as `caret`. The mirror holds the whole text, so it wraps exactly like
 * the textarea.
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
    const { selectionStart: at, selectionEnd, value } = input;
    // A selection is shown by its highlight instead.
    if (document.activeElement !== input || at !== selectionEnd) {
      caret.hidden = true;
      return;
    }
    // The marker is the character the caret is drawn over. At the end of a line the caret goes
    // just after the line's last character, where the browser puts its own: a stand-in
    // character there could wrap to the next line on its own.
    const marker = document.createElement("span");
    const under = value[at];
    const last = value[at - 1];
    const atLineEnd =
      (under === undefined || under === "\n") && last !== undefined && last !== "\n";
    if (atLineEnd) {
      // An emoji is two UTF-16 units: the marker takes both.
      const from = /[\uDC00-\uDFFF]/.test(last) && at >= 2 ? at - 2 : at - 1;
      marker.textContent = value.slice(from, at);
      mirror.replaceChildren(value.slice(0, from), marker, value.slice(at));
    } else {
      // Over an empty field the caret covers the first letter of the placeholder, like a
      // terminal; on an empty line it covers a space.
      const shown = value === "" ? input.placeholder[0] : under;
      marker.textContent = shown && shown !== "\n" ? shown : NBSP;
      const rest = under && under !== "\n" ? value.slice(at + 1) : value.slice(at);
      mirror.replaceChildren(value.slice(0, at), marker, rest);
    }

    // A hidden caret has no offset parent: shown first, it is placed in its own container
    // rather than in the page, where it would stretch the page past the window.
    caret.hidden = false;
    const lineHeight = parseFloat(getComputedStyle(input).lineHeight);
    const rect = marker.getBoundingClientRect();
    const box = (caret.offsetParent ?? document.body).getBoundingClientRect();
    // Spaces may hang past the end of a full line; the browser keeps its caret at the edge of
    // the text there, so the block caret never reaches beyond the padding.
    const mirrorStyle = getComputedStyle(mirror);
    const edge =
      mirror.getBoundingClientRect().right -
      parseFloat(mirrorStyle.paddingRight) -
      parseFloat(mirrorStyle.borderRightWidth);
    const left = Math.min(atLineEnd ? rect.right : rect.left, edge);
    // The marker box is the glyph height; centre the caret on the whole line instead.
    const lineTop = rect.top - (lineHeight - rect.height) / 2;
    caret.textContent = atLineEnd ? NBSP : marker.textContent;
    caret.style.left = `${left - box.left}px`;
    caret.style.top = `${lineTop - box.top - input.scrollTop}px`;
    // After the last character the caret is one space wide, which its own space gives it.
    caret.style.width = atLineEnd ? "" : `${rect.width}px`;
    caret.style.height = `${lineHeight}px`;
    caret.style.lineHeight = `${lineHeight}px`;
  }

  document.addEventListener("selectionchange", update);
  input.addEventListener("focus", update);
  input.addEventListener("blur", update);
  return update;
}
