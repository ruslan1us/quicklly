/**
 * happy-dom has no `document.execCommand`, which the app uses to change text fields so Ctrl+Z
 * can undo it. This stand-in does the two commands the app uses on the focused text field:
 * `insertText` types its value over the selection, `delete` removes the selection. Like a
 * browser, it leaves the caret after the change and fires an `input` event. Tests type with it
 * too (see `type` in `helpers.ts`).
 */
document.execCommand = (command: string, _showUI?: boolean, value = "") => {
  const field = document.activeElement;
  if (!(field instanceof HTMLTextAreaElement) || !["insertText", "delete"].includes(command)) {
    return false;
  }
  const { selectionStart: start, selectionEnd: end, value: text } = field;
  const inserted = command === "insertText" ? value : "";
  field.value = text.slice(0, start) + inserted + text.slice(end);
  field.setSelectionRange(start + inserted.length, start + inserted.length);
  field.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: command }));
  return true;
};
