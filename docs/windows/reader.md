# Reader

The reader browses the notes files and the notes in them, and is where existing notes are changed:
marked done, edited inline or in the Pad, deleted, and the changes undone. It also copies a note's
links and opens its images. It lists files as a tree or by last change, shows progress per file and
how long each done note has left before it is archived. A pinned reader stays on screen, follows
changes as they happen and reopens on the same file. User-facing keys are in the root README,
[Reader](../../README.md#reader).

## Files

| File | Role |
| --- | --- |
| [`src/windows/reader/index.html`](../../src/windows/reader/index.html) | Header with title and pin, rows, empty and error lines, footer |
| [`src/windows/reader/reader.ts`](../../src/windows/reader/reader.ts) | Behaviour, including the inline editor and the undo stack |
| [`src/windows/reader/reader.css`](../../src/windows/reader/reader.css) | Layout; imports `terminal.css` and `calendar.css` |
| [`src-tauri/src/reader.rs`](../../src-tauri/src/reader.rs) | Listing, parsing, changes, undo |
| [`src-tauri/src/reader_window.rs`](../../src-tauri/src/reader_window.rs) | Pin state, pinned file, bounds, sort order |

## Life cycle

- Opened with <kbd>←</kbd> in the empty input window (`open_reader`) or on a specific note
  (`open_note` from a search result or the reminders), through `show_popup`.
- On start the page loads the pin state and sort order, opens the pinned file if any, takes a
  pending target note, then shows itself: a pinned reader at its saved bounds, an unpinned one
  centred by `reveal()`. When the saved place is on no current monitor, a pinned reader gets
  only its saved size and stays where the new window was created, as with nothing saved.
- An unpinned reader closes when it loses focus (only once visible). <kbd>Esc</kbd> or
  <kbd>Ctrl</kbd>+<kbd>C</kbd> closes it. Closing destroys the window and with it the undo history.
- The view reloads on `focus` and on `notes-changed`, unless a note is being edited.
  `notes-changed` also comes from the notes folder watcher, so a pinned reader follows notes saved
  through the CLI or changed in another editor; after the reader's own changes it arrives a second
  time, about 300 ms later, and the view simply reloads again.

## Views and keys

**File list.** Each row: pointer, name (daily files without `.md`), `today` for today's daily file,
an eight-cell progress bar, and `done/total`, `✓ all done` or `empty`. The bar is full only when
every note is done and shows at least one cell once any note is. By name, folders appear once as
`folder/` rows above their files, which are indented; recent first, files show their full path.

| Key | Action | Command |
| --- | --- | --- |
| <kbd>↑</kbd> <kbd>↓</kbd> | Select (wraps) | |
| <kbd>Enter</kbd>, double-click | Open the file | `read_note_file` |
| <kbd>Tab</kbd> | Toggle sort recent first / by name, keeping the selected file | `set_reader_sort` |
| <kbd>Del</kbd> twice | Move the file and its unused images to the Recycle Bin | `delete_note_file` |
| <kbd>→</kbd> | Hide, show the input window, close | `open_input` |
| <kbd>Ctrl</kbd>+<kbd>Z</kbd> | Undo the latest note change | `undo_change` |

**Notes of a file.** Day headings as `── YYYY-MM-DD`; each note: pointer, time, `✓`, text (links
shortened, images as labels that wrap as a whole, dates in the date format, done date hidden) and, for done notes,
`archive in N days`, `archive tomorrow` or `archive today` (from `get_archive_days`). A newly
opened file selects its latest note.

| Key | Action | Command |
| --- | --- | --- |
| <kbd>↑</kbd> <kbd>↓</kbd> | Select a note (wraps; headings skipped) | |
| <kbd>Space</kbd> | Toggle done; a note marked not done becomes `- [ ] HH:MM` (see [Data format](../data-format.md#dates)) | `set_note_done` |
| <kbd>Enter</kbd> | Edit inline | `edit_note` |
| <kbd>Shift</kbd>+<kbd>Enter</kbd> | Edit in the Pad | `edit_in_pad` |
| <kbd>C</kbd> | Copy the note's links, whole, one per line | |
| <kbd>O</kbd> | Open the note's images in the default viewer; only pasted images (right in `images/`, with an image extension) open, any other link shows an error and opens nothing | `open_images` |
| <kbd>Del</kbd> twice | Delete the note and its unused images | `delete_note` |
| <kbd>Ctrl</kbd>+<kbd>Z</kbd> | Undo the latest change made in this reader | `undo_change` |
| <kbd>Ctrl</kbd>+<kbd>P</kbd> | Pin or unpin | `set_reader_pin` |
| <kbd>←</kbd> | Back to the file list, with this file selected | |

On macOS <kbd>Ctrl</kbd>+<kbd>Z</kbd> and <kbd>Ctrl</kbd>+<kbd>P</kbd> are ⌘Z and ⌘P, and
<kbd>⌫</kbd> works as <kbd>Del</kbd> (Mac keyboards have no forward Delete);
<kbd>Ctrl</kbd>+<kbd>C</kbd> still closes (see [Frontend](../frontend.md#editing-conventions)).

While a delete waits for confirmation the footer says so; any other key or a click cancels it.

## Behaviour

- **Inline editor.** <kbd>Enter</kbd> replaces the note's text with a textarea that has the block
  caret, `/date` with the calendar and whole-date deletion. <kbd>Enter</kbd> saves,
  <kbd>Shift</kbd>+<kbd>Enter</kbd> adds a line, <kbd>Esc</kbd> cancels and reloads the file. A
  blank or unchanged text just ends the edit. If the save is refused, the editor stays open with the
  text and the error shows. The editor has no tag suggestions and does not accept pasted images. The
  calendar opens under the note: the window refits (`fit`) as it shows, changes or hides, and then
  the calendar is scrolled into view (`scrollIntoView({ block: "nearest" })`), so under the last
  note, in a long file or in a pinned reader it is not left below the bottom of the window.
- **Change guard.** Every change sends the note as displayed (`NoteRef`). If the file changed in
  the meantime, Rust refuses, the error stays visible and the file is reloaded (for an inline edit,
  once the edit ends). See
  [Backend](../backend.md#changes-to-existing-notes).
- **Undo.** Each successful change pushes `{ what, name, line, change }` onto an in-memory stack.
  <kbd>Ctrl</kbd>+<kbd>Z</kbd> pops the latest, asks Rust to restore the file (refused if the file
  is no longer exactly as the change left it) and its images, opens that file if another one is
  shown, selects the note and reports `Undone: delete`, `Undone: done` or `Undone: edit`, plus
  `couldn't bring back N images` when some images were no longer in the Recycle Bin. The entry is
  dropped even if the undo fails, so the next press moves on to the one before. A change whose
  images couldn't all be moved to the Recycle Bin is still saved and pushed: those images stay in
  `images/` and the failure is only logged. Inside the inline editor <kbd>Ctrl</kbd>+<kbd>Z</kbd> is
  the editor's own text undo.
- **Pinning.** A file must be open to pin. Pinning saves the pin state and the open file and makes
  the window resizable (minimum 440×200 at 100 % scale); moves and resizes are saved. A pinned
  reader keeps its size, scrolls its rows between a fixed header and footer, and reopens on its
  file. Unpinning refits and centres the window.
- **Sizing.** Unpinned, the window fits its content up to 520 CSS pixels; beyond that the body
  scrolls. Selecting the first or last row scrolls fully so the header or footer shows too.

## What is not undoable

Deleting a whole file (it is in the Recycle Bin), changes made in the Pad or the reminders, and
anything done in an earlier reader session are not in the undo stack. A change cannot be undone
once anything else has changed the file, including a note added from the input window or the
archive moving notes at a day change. See [ADR 0011](../decisions/0011-undo-via-whole-file-snapshots.md).
