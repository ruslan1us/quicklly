# Reminders

The reminders window lists the notes that are not done and whose due date (`📅`) is today or
earlier, oldest first. It appears on its own in the bottom-right corner when the application
starts and when a new day begins, without taking the keyboard from the application in use. A
second list, "ahead", shows the notes with a later date grouped by day. Notes can be marked done,
snoozed or opened in the reader. User-facing keys are in the root README,
[Reminders](../../README.md#reminders).

## Files

| File | Role |
| --- | --- |
| [`src/windows/reminders/index.html`](../../src/windows/reminders/index.html) | Title, rows, empty and error lines, footer |
| [`src/windows/reminders/reminders.ts`](../../src/windows/reminders/reminders.ts) | Behaviour |
| [`src/windows/reminders/reminders.css`](../../src/windows/reminders/reminders.css) | Layout; imports `terminal.css` |
| [`src-tauri/src/reminders.rs`](../../src-tauri/src/reminders.rs) | Daily round, window creation and placement, `open_reminders`, `list_due`, `reveal_reminders`, `place_reminders` |
| [`src-tauri/src/reader.rs`](../../src-tauri/src/reader.rs) | `due`, `snooze` |

## Life cycle

- **Automatic.** The daily-round thread runs at start and whenever the local date changes (checked
  every 60 seconds). It first archives done notes, then, if any note is due, shows the reminders
  with `focus = false`. If the window is already open it only receives `notes-changed` and reloads.
- **On request.** `/reminders` calls `open_reminders`, which returns `false` (the input window shows
  "Nothing due") when there is nothing due and nothing ahead; otherwise it shows the window with
  `focus = true`, or focuses an open one.
- **Showing.** The window is created hidden and unfocused. The page loads the due list; if it is
  empty it switches to the ahead list; if that is empty too it closes without ever showing. It then
  fits its content (up to 400 CSS pixels) and calls `reveal_reminders`, which places it in the
  bottom-right corner of the active monitor's work area, 16 pixels from the edges, shows it and
  focuses it only when it was requested. A window shown without focus does not activate; a click
  gives it the keyboard. On macOS, where Quicklly hides itself when its last window goes (see
  [Focus on macOS](../architecture.md#focus-on-macos)), `reveal_reminders` first unhides the app:
  with `come_forward` when the window takes the focus, and without activating it
  (`unhideWithoutActivation`) when it doesn't.
- It does not close on focus loss. It closes on <kbd>Esc</kbd>, <kbd>Ctrl</kbd>+<kbd>C</kbd>, and by
  itself when the last note leaves the list it is showing.

## Keys and commands

| Key | Due list | Ahead list | Command |
| --- | --- | --- | --- |
| <kbd>↑</kbd> <kbd>↓</kbd> | Select | Select | |
| <kbd>Space</kbd> | Mark done | Mark done | `set_note_done` |
| <kbd>S</kbd> | Move the date to tomorrow | | `snooze_note(days: 1)` |
| <kbd>W</kbd> | Move the date to a week from today | | `snooze_note(days: 7)` |
| <kbd>Enter</kbd> | Open in the reader | Open in the reader | `open_note` |
| <kbd>Tab</kbd>, <kbd>Shift</kbd>+<kbd>Tab</kbd> | Show ahead | Show due | `list_due` |
| <kbd>Esc</kbd> | Close | Close | |

Snoozing is not offered ahead, where it would move a note's date earlier.

## Behaviour

- **Rows** show the first line of the note (links shortened, image labels, dates in the date
  format) and `today`, `N days overdue` (in the error colour), `tomorrow` or `in N days`. In the
  ahead list each day starts with a `── <date>` row.
- **Selection of notes.** `reader::due` takes the first valid `📅` date of each not-done note in
  every notes file (not the archive). A due note is one whose date is today or earlier; ahead,
  later than today. Sorting is by date with a stable sort, so notes of one date keep file and line
  order.
- **Snooze** rewrites only that first date (`reader::snooze`) through the same guarded edit as the
  reader; the note's other dates and text stay as they are.
- **Opening a note** hides this window, calls `open_note`, then closes this window.
- **Following changes.** The window reloads on `notes-changed`: changes made in the reader, the
  archive, and anything the notes folder watcher sees (the CLI, another editor, and a second time
  the app's own writes). It closes when the reload empties a list that had notes.
- **Corner placement after resizing.** Every refit calls `place_reminders` so the window keeps its
  bottom-right position as its height changes.

## Edge cases

- <kbd>Tab</kbd> to an empty list keeps the window open and shows "Nothing ahead." or
  "Nothing due."; the window closes only when a list that had notes becomes empty.
- Changes made here are not in the reader's undo history.
- A note with a due date and no other text is listed with an empty text.
