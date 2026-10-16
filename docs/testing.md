# Testing

Quicklly has two automated test suites: Vitest tests for the frontend, which run the real window
pages in a simulated DOM with the Rust side replaced by a mock, and Rust unit tests for the note
format, file operations and parsing. Neither suite starts the application, touches the user's
notes or settings, or uses the real Recycle Bin. Behaviour that depends on native windows, focus,
the hotkey or the installer is verified by hand. Both suites run in CI on every push and pull
request.

## Frontend tests

| Item | Value |
| --- | --- |
| Runner | Vitest ([`vitest.config.ts`](../vitest.config.ts)), `npm test` |
| Environment | `happy-dom` for every test |
| Location | `tests/**/*.test.ts` |
| Set-up | [`tests/setup.ts`](../tests/setup.ts) |
| Helpers | [`tests/helpers.ts`](../tests/helpers.ts) |

### Set-up

happy-dom does not implement `document.execCommand`, which the application uses for every
programmatic text change (see [Frontend](frontend.md#editing-conventions)). `setup.ts` installs a
stand-in that supports the two commands the application uses on the focused textarea:
`insertText` replaces the selection with the value, `delete` removes the selection; both leave the
caret after the change and dispatch an `input` event, like a browser.

### Helpers

| Helper | Purpose |
| --- | --- |
| `mockTauri(replies)` | Calls `mockWindows("main")` and `mockIPC` from `@tauri-apps/api/mocks`; records every command in the returned list and answers from `replies` (a value, or a function of the arguments that may throw). Answers `plugin:window|scale_factor` and `plugin:window|inner_size` by default so pages can fit themselves. Events are mocked too (`shouldMockEvents`): `emit` from `@tauri-apps/api/event` reaches the page's `listen` handlers, as an event from Rust would. |
| `argsOf(calls, cmd)` | The arguments of each call of a command |
| `loadPage(html, load)` | Puts the `<body>` of a page's `index.html` (imported with `?raw`) into the document, resets modules, imports a fresh copy of the page script and waits for it to settle; spies on listeners added to `document` and `window` |
| `closePage()` | Removes those listeners and empties the body, so the next test starts clean |
| `textField(value)` | A focused textarea with the caret at the end, for testing shared modules alone |
| `type(field, text)` | Types through `execCommand("insertText")`, as the keyboard would |
| `press(target, key, init)`, `keydown(key, init)` | Dispatch a cancellable `keydown`; the returned event shows whether it was handled |
| `settle()` | Lets pending promises finish; mocked commands answer immediately |

Tests that depend on the date fix it with `vi.setSystemTime`. Letter shortcuts are pressed with a
`code` as well as a `key` so that layout independence is tested (for example `key: "с"`,
`code: "KeyC"`).

### Coverage

| File | Covers |
| --- | --- |
| `links.test.ts` | Link recognition, trailing punctuation and brackets, `shortLink`, `splitLinks` |
| `dates.test.ts` | Formatting and parsing in every format, leap years, `toStored`/`toDisplay` round trips, `/date` detection, `commandDate`, `splitDoneDate`, whole-date deletion |
| `images.test.ts` | Label and Markdown conversion, renumbering clashes, `imagePaths`, pasting (saved, numbered, held, text wins, errors), `releaseImages` |
| `calendar.test.ts` | `/date` resolution and the calendar keys |
| `tagSuggest.test.ts` | Filtering, picking with Enter and Tab, wrapping, Esc, nested tags, replacing a tag in the middle |
| `main.test.ts` | Input window: Enter saves and hides, Shift+Enter, dates on Enter, calendar, date format, search results (the query marked in the shown text, the day in the date format or blank), pasted images let go of when the note is cleared or handed to the Pad with Ctrl+E, the note kept when Ctrl+E fails |
| `pad.test.ts` | Pad: the images let go of when a note is saved, when an edit replaces the draft and when it ends; the draft's images held, also after text moved from the input window; a draft that can't be read shown as an error and never saved over |
| `reader.test.ts` | File list order and tree, opening a file, Tab sort, copying links, opening images, inline edit saved and refused, Ctrl+Z in its cases |
| `reminders.test.ts` | Due list, snooze keys, closing when empty, the ahead list and Tab |
| `done.test.ts` | Grouping by day, empty state, copying Markdown, reloading on `notes-changed`, Esc |
| `help.test.ts` | Version in the header, the Archive text following the setting |

Not covered by automated tests: the Pad beyond its pasted images, settings and changelog pages,
the rest of the help, and search in the input window beyond how its results are shown.

## Rust tests

Unit tests live in a `#[cfg(test)] mod tests` at the bottom of each module and run with
`cargo test` in `src-tauri/`.

| Module | Covers |
| --- | --- |
| `notes` | File choice by mode and tag, headings, nested tags and image links, missing trailing newline, normalisation of lines, an unreadable target file named in the error |
| `reader` | Parsing, listing order and tree, tags, path validation, search (only date headings as days, undated notes last), due and ahead, done journal, done marks and dates, edit, delete, snooze, removal of empty day headings (other headings kept), undo, refusing stale changes, line endings, a file that is not UTF-8 listed with no notes and left out of search, due and the done journal |
| `archive` | Selection by done date, daily, grouped, tag, hand-written and nested files, empty headings that are no day kept, appending to an existing archive, notes and archive files that can't be read left alone |
| `images` | Naming and numbering, extensions, path extraction, relinking, safe resolution (only pasted images right in `images/` with an image extension), unused-image detection (archive and nested files included), pasted images nothing mentions (notes, archive, tag folders, kept texts, any link form, case) and only files right in `images/`, failing with the file's name when a notes file can't be read, going on after an image that can't be moved (with a stand-in for the Recycle Bin), Recycle Bin item matching |
| `settings` | Serialised strings of date formats and archive periods; reading the notes folder and mode from a settings file |
| `hotkey` | Parsing, labels, round trip of the stored form |
| `pad` | Draft file: missing reads as empty, appending text, an unreadable draft an error and left unchanged |
| `cli` | Request parsing for `add` and links, decoding, rejection of other input, relative file names |
| `updater` | Which cache files count as stale downloads (temporary files do not), their removal from a folder, saving a download through a temporary file and removing it when the save fails |
| `watcher` | Which paths count as a notes change; a real watcher on a temporary folder reports a `.md.tmp` renamed into place and ignores images and temporary files |

### Rules

- **Pure functions.** Logic is written as functions over a folder path, text and dates, so tests
  need no `AppHandle`, no window and no settings store. Commands stay thin wrappers.
- **Temporary folders.** A test creates its own folder under the system temporary directory, named
  `quicklly-test-<name>-<process id>`, removes any leftover first and deletes it at the end. Tests
  never use the real notes folder or `%APPDATA%`.
- **Fixed dates.** Dates are constructed explicitly (`NaiveDate::from_ymd_opt`); nothing depends on
  the current date or time zone.
- **No real Recycle Bin.** Moving files to the Recycle Bin and restoring them is not exercised.
  Tests cover what decides it: path validation (`trash_file` refuses anything but a notes file,
  before any file is touched), the selection of unused images (`unused`) and of unsaved pasted
  ones (`unsaved`), moving several images one by one (`trash_each`, given a stand-in for
  `trash::delete`), and the choice of Recycle Bin entries to restore (`to_restore`) on hand-built
  `TrashItem` values.
- **Exact file content.** File tests assert the whole resulting text, including blank lines and
  line endings.

## Manual verification

These are checked by hand in `npm run tauri dev` or an installed build before a release:

- Hotkey registration and change, the tray menu, start with Windows.
- Window appearance, focus behaviour (no focus stolen at start, reminders shown without focus),
  hiding on focus loss, pinning, bounds restore.
- Transparency and scale, the three themes, layout in the release build.
- Placement on the monitor in use with several monitors and different scalings.
- Deleting to and restoring from the Recycle Bin, opening images in the default viewer.
- `quicklly add` from PowerShell and `cmd`, and `quicklly://` links after installation.
- Open reader, reminders and done windows following a note saved through the CLI and a file
  edited in another editor; the watcher following a new notes folder chosen in settings.
- Updates: a release found, downloaded, announced, installed at start and by `/update`.
