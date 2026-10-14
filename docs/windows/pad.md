# Pad

The Pad is a larger editor for longer notes: line numbers, highlighting of list markers, tags,
links, images and dates, list continuation and indentation, and a draft that survives closing the
window and restarting the application. <kbd>Ctrl</kbd>+<kbd>Enter</kbd> saves the text as one note,
exactly as the input window would. The Pad also edits existing notes sent from the reader. It can
be pinned to stay open, movable and resizable. User-facing keys are in the root README,
[Pad](../../README.md#pad).

## Files

| File | Role |
| --- | --- |
| [`src/windows/pad/index.html`](../../src/windows/pad/index.html) | Header with title and pin, gutter, editor area with backdrop, textarea, caret and mirror, tag list, calendar, footer |
| [`src/windows/pad/pad.ts`](../../src/windows/pad/pad.ts) | Behaviour |
| [`src/windows/pad/pad.css`](../../src/windows/pad/pad.css) | Layout; imports `calendar.css` |
| [`src-tauri/src/pad.rs`](../../src-tauri/src/pad.rs) | Window creation, placement, pin state and bounds, draft file, `expand_to_pad`, `edit_in_pad` |

## Life cycle

- `pad::show` creates the window on first use (560×380 logical pixels times the scale, hidden),
  and on every show arranges it: a pinned Pad goes back to its saved bounds; an unpinned one gets
  the default size and the corner from **Pad position**, 16 pixels from the edges of the active
  monitor's work area.
- The page shows the window itself once the theme, the date format, the draft and the pin state
  are loaded and any pending edit is taken.
- Focus loss hides an unpinned Pad. <kbd>Esc</kbd> saves the draft and hides. The window is never
  destroyed, so the text and its undo history survive hiding.

## Keys and commands

| Key | Action | Command |
| --- | --- | --- |
| <kbd>Ctrl</kbd>+<kbd>Enter</kbd> | Save the text as a note (or save the edited note) | `save_note` / `edit_note` |
| <kbd>Enter</kbd> | Resolve a `/date`; in a list item, start the next item; in an empty item, end the list; otherwise a new line | |
| <kbd>Tab</kbd>, <kbd>Shift</kbd>+<kbd>Tab</kbd> | Indent or outdent by two spaces (all selected lines when the selection spans lines) | |
| <kbd>Ctrl</kbd>+<kbd>P</kbd> | Pin or unpin | `set_pad_pinned` |
| <kbd>←</kbd> in an empty Pad (not editing) | Back to the input window; an unpinned Pad hides first | `open_input` |
| <kbd>Esc</kbd>, <kbd>Ctrl</kbd>+<kbd>C</kbd> without selection | Cancel an edit, save the draft, hide | `set_pad_draft` |

List items are lines starting with optional indentation and `- [ ] `, `- [x] `, `- `, `* ` or
`1. `; a checkbox item continues as `- [ ] `, a numbered one with the next number.

## Behaviour

- **Draft.** The draft is saved in stored form 300 ms after typing stops and on hide, to
  `pad-draft.md` in the app data folder (`set_pad_draft`); an empty draft deletes the file. Saving a
  note empties the Pad; an unpinned Pad then hides, a pinned one shows `saved ✓` in the status bar
  for 1.5 seconds.
- **Moving text from the input window.** `expand_to_pad` appends the input text to the draft on a
  new line (or replaces a blank draft) on the Rust side, then shows the Pad; an open Pad reloads the
  draft on `pad-draft-changed`.
- **Editing a reader note.** <kbd>Shift</kbd>+<kbd>Enter</kbd> in the reader calls `edit_in_pad`,
  which stores the note in `PendingEdit` and emits `pad-edit` if the Pad exists; the Pad takes it
  with `take_pad_edit`, saves its draft, and shows the note without its done date. The title shows
  `editing HH:MM`. <kbd>Ctrl</kbd>+<kbd>Enter</kbd> sends `edit_note` with the note as the reader
  saw it, so the change is refused if the file changed meanwhile; <kbd>Esc</kbd> cancels. Either way
  the draft comes back.
- **Rendering.** Each logical line is rendered in the backdrop with its list marker, checkbox, a
  struck-through done item, `#tags`, links, image labels and dates highlighted. The gutter shows one
  number per logical line, as tall as the rows it wraps to; the current line is highlighted. The
  textarea has no scrollbar so that it wraps exactly like its copies.
- **Status bar.** `Ln N, Col N · N chars`, or the last error.
- **Tag suggestions and the calendar** appear above the footer and take space from the text; the
  line being typed is scrolled back into view.
- **Scale.** After a scale change an unpinned Pad gets the default size at the new scale; a pinned
  one keeps its size (`pad::rescale`).

## Edge cases

- A blank text is not saved, and a blank edit is not sent (deleting is done in the reader).
- An edit made in the Pad is not recorded in the reader's undo history.
- The draft is also the hand-over buffer for <kbd>Ctrl</kbd>+<kbd>E</kbd>; text moved there while
  a reader note is being edited appears once the edit ends.
- The Pad window is created without `focused(false)`; it is only created on an explicit user action,
  never at start.
