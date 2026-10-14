# Input window

The input window is the core of the application: a one-line field that appears over every other
window on the hotkey, saves the typed note on <kbd>Enter</kbd> and disappears. It grows with
multi-line notes, recalls earlier notes, runs `/` commands, searches all notes, suggests tags,
inserts dates and accepts pasted images. It is created once at start and only hidden afterwards,
so it appears without delay. User-facing keys are in the root README, [Usage](../../README.md#usage).

## Files

| File | Role |
| --- | --- |
| [`src/windows/input/index.html`](../../src/windows/input/index.html) | Textarea, backdrop, prompt, hint, results, tag list, calendar, message, caret and mirrors |
| [`src/windows/input/main.ts`](../../src/windows/input/main.ts) | All behaviour |
| [`src/windows/input/styles.css`](../../src/windows/input/styles.css) | Layout; imports `calendar.css` |
| [`src-tauri/tauri.conf.json`](../../src-tauri/tauri.conf.json) | Declares the `main` window: 640×56, hidden, unfocused, transparent, frameless, always on top, no taskbar button |
| [`src-tauri/src/lib.rs`](../../src-tauri/src/lib.rs) | `show_input`, focus-loss and close handling, `save_note`, `search_notes`, `open_*` commands |

## Life cycle

- `show_input` centres the window on the active monitor, shows and focuses it. It runs on the
  hotkey (when **Hotkey opens** is the quick line), a tray click, tray **New note**, a second launch
  of the executable, and `open_input` from the reader or the Pad.
- Focus loss hides the window from Rust without clearing it, so a click elsewhere keeps the draft
  for the next show. <kbd>Esc</kbd>, a saved note or a command clears the field, leaves search mode
  and hides. Alt+F4 hides instead of closing.
- On every `focus` event the page refocuses the textarea, reloads the tag list and, if a release is
  waiting, shows `update vX available · /update` in the hint for two seconds.

## Keys and commands

| Key or text | Action | Command |
| --- | --- | --- |
| <kbd>Enter</kbd> | Resolve a `/date` first; otherwise run a `/` command, open the selected search result, or save the note | `save_note` |
| <kbd>Shift</kbd>+<kbd>Enter</kbd> | New line | |
| <kbd>Esc</kbd>, <kbd>Ctrl</kbd>+<kbd>C</kbd> without selection | Clear and hide | |
| <kbd>↑</kbd> / <kbd>↓</kbd> in an empty field or while showing an unedited history entry | Walk the history | |
| <kbd>←</kbd> in an empty field | Open the reader | `open_reader` |
| <kbd>→</kbd> in an empty field (not searching) | Open the Pad | `open_pad` |
| <kbd>Ctrl</kbd>+<kbd>E</kbd> (not searching) | Move the text into the Pad draft | `expand_to_pad` |
| `?` typed into an empty field | Enter search mode | `search_notes` |
| <kbd>Backspace</kbd> on an empty query | Leave search mode | |
| `/config`, `/pad`, `/help`, `/changelog`, `/done` | Open that window | `open_settings`, `open_pad`, `open_help`, `open_changelog`, `open_done` |
| `/reminders` | Open the reminders focused, or show "Nothing due" | `open_reminders` |
| `/update` | Install a newer release now, or show "Quicklly is up to date (vX)" | `install_update` |
| `/exit` | Quit | `exit_app` |

A command is recognised only when the whole trimmed note equals it; `/config later` is saved as a
note. Commands that open another window hide the input window first (`switchTo`), so the two never
show together.

## Behaviour

- **Sizing.** The textarea's height is measured on an invisible mirror with a zero-width space
  appended (so a trailing newline counts as a line once something follows it). The window grows to
  ten lines (56 px plus 26 px per extra line, times the zoom) and the note scrolls beyond that.
  Search results, tag suggestions, the calendar and the message line below the note add their
  heights. The window is resized only when the total changes.
- **Rendering.** The textarea's text is transparent; a backdrop renders the same text with links
  underlined and image labels and dates coloured, and a block caret replaces the native one (see
  [Frontend](../frontend.md#editing-conventions)).
- **History.** Notes saved during the session (up to 100, in stored form, consecutive duplicates
  skipped) are browsed like a shell history. The history is kept in memory only and is lost when
  the application quits.
- **Search mode.** A `?` typed into an empty field switches to search: the `?` becomes a prompt,
  results (day, time, done mark, first line with further lines joined by ` ⏎ `) appear under the
  field with the query highlighted, ↑ ↓ select, <kbd>Enter</kbd> or a click opens the result in the
  reader through `open_note`. Each search is numbered so that a slower earlier response cannot
  replace a newer one. Search covers notes files only, not the archive, and matches the stored text
  (see [Data format](../data-format.md#display-and-stored-forms)). Tag suggestions and image paste
  are disabled in search mode.
- **Dates.** `/date` and <kbd>Enter</kbd> open the calendar under the note; `/date tomorrow` or
  `/date <date in the format>` inserts the date at once; an unknown word shows
  `Unknown date: … — use <format> or tomorrow`. One <kbd>Backspace</kbd> after a date removes it as
  a whole. When the date format changes, the text in the field is converted to the new format.
- **Images.** <kbd>Ctrl</kbd>+<kbd>V</kbd> with an image (and no text) in the clipboard saves it at
  once through `save_image` and types `[Image #N]`. An image saved this way stays in `images/` even
  if the note is then cancelled.
- **Messages.** Errors and command answers appear on a line under the note for four seconds; an
  error also outlines the field. Typing clears them.

## Edge cases

- <kbd>←</kbd> opens the reader from an empty search query as well, while <kbd>→</kbd> is disabled
  in search mode.
- A blank note just hides the window; nothing is written.
- The history holds what was saved, so recalling an entry after the date format changed shows the
  dates in the current format.
