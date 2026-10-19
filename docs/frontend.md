# Frontend

The frontend is plain TypeScript and CSS without a UI framework, built by Vite as one page per
window. Each window lives in its own folder under [`src/windows/`](../src/windows/); code and
styles shared by several windows live in [`src/shared/`](../src/shared/). All windows share one
visual language (a terminal-style popup with a monospace font, a block caret and a key-hint
footer) and a few conventions that keep editing behaviour identical everywhere: the same tag
suggestions, the same `/date` calendar, the same link and image handling, and native undo for
every programmatic text change. This page describes the build set-up, the shared modules and those
conventions.

## Build set-up

| Item | Value |
| --- | --- |
| Vite root | `src/` ([`vite.config.ts`](../vite.config.ts)) |
| Pages | `src/windows/<name>/index.html`, listed in `build.rolldownOptions.input` |
| Output | `dist/`, so a page is served at `windows/<name>/index.html` (Rust: `page(name)`) |
| Dev server | Port 1420, `strictPort`; `optimizeDeps.entries` lists the pages so dependencies are pre-bundled |
| TypeScript | `strict`, `noUnusedLocals`, `noUnusedParameters`; `npm run build` runs `tsc` before `vite build` |
| Dependencies | `@tauri-apps/api` (IPC, events, window), `@tauri-apps/plugin-opener` |
| Raw imports | `?raw` (typed by `vite/client`): the changelog window imports `CHANGELOG.md`, tests import pages |
| Platform | `define` puts the build's `process.platform` in for `__PLATFORM__` (declared in `src/vite-env.d.ts`), read by [`platform.ts`](../src/shared/platform.ts); each system builds its own pages |

Each window folder holds `index.html`, one TypeScript entry and, for most windows, one stylesheet.
The page links `../../shared/themes.css` and its own stylesheet. See
[ADR 0009](decisions/0009-one-folder-per-window.md).

| Folder | Entry | Stylesheet | Page |
| --- | --- | --- | --- |
| `input` | `main.ts` | `styles.css` | [Input window](windows/input.md) |
| `pad` | `pad.ts` | `pad.css` | [Pad](windows/pad.md) |
| `reader` | `reader.ts` | `reader.css` | [Reader](windows/reader.md) |
| `reminders` | `reminders.ts` | `reminders.css` | [Reminders](windows/reminders.md) |
| `done` | `done.ts` | `done.css` | [Done](windows/done.md) |
| `settings` | `settings.ts` | none (links `terminal.css`) | [Settings](windows/settings.md) |
| `help` | `help.ts` | `help.css` | [Help and changelog](windows/help-and-changelog.md) |
| `changelog` | `changelog.ts` | `changelog.css` | [Help and changelog](windows/help-and-changelog.md) |

## Shared modules

| Module | Responsibility | Used by |
| --- | --- | --- |
| [`platform.ts`](../src/shared/platform.ts) | The system the pages were built for: `platform` (`win32`, `darwin` or `linux`), `isWindows`, `isMac`, `isLinux`; `primary(e)`, the modifier of the app's own shortcuts (⌘ on macOS, <kbd>Ctrl</kbd> elsewhere); `names`, what the hints call keys and things on this system (`mod`, `ctrl`, `shift`, `del`, `trash`, `cli`); `nameThings()`, which puts them into a page's elements marked `data-name` | Input, Pad, reader, settings, help |
| [`theme.ts`](../src/shared/theme.ts) | Loads the appearance (`get_appearance`) and follows `appearance-changed`: sets `data-theme`, the `--opacity` variable and the exported `zoom` | Every page |
| [`popup.ts`](../src/shared/popup.ts) | `fitToContent(maxHeight)` sizes a popup to its `main` element at the base width (640 logical pixels × zoom); `reveal()` centres, shows and focuses it | Reader, reminders, done, settings, help, changelog |
| [`dates.ts`](../src/shared/dates.ts) | Date format, `toStored`/`toDisplay`, `/date` detection, `splitDoneDate`, `DONE_DAYS` (the done journal's period), `dateNodes` highlighting, `deleteDatesWhole`, `followDateFormat` | Input, Pad, reader, reminders, done, help |
| [`calendar.ts`](../src/shared/calendar.ts), [`calendar.css`](../src/shared/calendar.css) | `datePicker`: Enter on `/date` resolves it or opens the keyboard calendar, and types the picked date over the command | Input, Pad, reader editor |
| [`tagSuggest.ts`](../src/shared/tagSuggest.ts) | Suggests existing tags (`list_tags`) for a `#tag` being typed; ↑ ↓ choose, Enter or Tab pick, Esc closes until the typed tag changes; exports `TAG_NAME`, the tag rule of `notes::first_tag`, which the Pad also highlights tags by | Input, Pad |
| [`images.ts`](../src/shared/images.ts) | Label ↔ Markdown conversion (`toMarkdown`, `fromMarkdown`), `imagePaths`, highlighting nodes, `pasteImages` (adds each pasted image to the window's held set), `releaseImages` (lets go of the held images when the text starts over, see [ADR 0018](decisions/0018-pasted-images-let-go-of-when-the-text-starts-over.md)) | Input, Pad, reader, reminders, done |
| [`links.ts`](../src/shared/links.ts) | Link recognition (`http(s)://…` or a bare domain with a known top-level domain), sentence-punctuation trimming, `shortLink`, `linkNodes` | Input, Pad, reader, reminders, done |
| [`blockCaret.ts`](../src/shared/blockCaret.ts) | Terminal-style block caret over a textarea whose native caret is hidden, positioned with a mirror element that holds the whole text (so it wraps like the textarea); at the end of a line it goes just after the last character, and never past the edge of the text | Input, Pad, reader editor |
| [`themes.css`](../src/shared/themes.css) | Theme colour variables, `--surface`, thin themed scrollbars | Every page |
| [`terminal.css`](../src/shared/terminal.css) | Popup base: font, body-scrolling layout, header, rows, error, footer | Popups |

## Styling conventions

- **Terminal style.** Monospace font (`Cascadia Mono`, then `Consolas`), a header
  `Quicklly · <window> · <context>`, a `❯` pointer on the selected row, and a footer that lists
  the keys separated by ` · `. Short confirmations ("Copied 2 links") appear in front of the
  footer for two seconds.
- **Themes.** `themes.css` defines `--bg`, `--fg`, `--dim`, `--accent`, `--caret`, `--error` for
  `default`, `defaultPlus` and `light`, selected with `<html data-theme>`. Pages use only these
  variables.
- **Transparency.** Rust gives every window an acrylic blur. The page background is `--surface`,
  `color-mix(in srgb, var(--bg) var(--opacity), transparent)`, where `--opacity` is
  `100 % − transparency`.
- **Scale.** Rust zooms each webview (`set_zoom`). Window sizes are set in logical pixels, so pages
  multiply CSS sizes by `zoom` from `theme.ts` when sizing their window, and refit on
  `appearance-changed`.
- **Popups scroll their body, not the window.** In `terminal.css` the `html` element does not
  scroll and the `body` does; the window's own scrollbar would get a black track in a transparent
  window. Window height is measured on `main`.
- **Import order.** A popup's stylesheet imports `terminal.css` with `@import` at the top instead
  of linking it separately; in release builds Vite may otherwise place the shared rules after the
  window's own and override them (this broke the reader's layout in 0.7.1). The settings window has
  no stylesheet of its own and links `terminal.css` directly.
- **Input window clipping.** The input window uses `body { overflow: clip }`, not `hidden`: the
  textarea grows a moment before the window does, and a scrollable body would scroll to the caret
  and back, making the text jump.

## Editing conventions

- **Programmatic text changes use `document.execCommand`.** Picking a tag, inserting a date or an
  image label, deleting a date as a whole, indenting and continuing lists in the Pad all call
  `execCommand("insertText" | "delete")` on the focused textarea. Unlike assigning `value`, this
  keeps the browser's native <kbd>Ctrl</kbd>+<kbd>Z</kbd> history intact. Tests provide a stand-in
  (see [Testing](testing.md)).
- **Highlighting backdrops.** Textareas cannot style parts of their text. The input window, the Pad
  and the reader editor draw the text twice: the textarea's own text is transparent and sits over a
  backdrop element with identical font, padding, width and wrapping, which renders the same text
  with links underlined, dates, image labels, tags and list markers coloured. Scrolling is mirrored
  with a `translateY` transform. Separate invisible mirror elements measure the text height and the
  caret position.
- **Display and stored forms.** Text fields hold the display form; the stored form is produced
  when text leaves the field. See [Data format](data-format.md#display-and-stored-forms).
- **Keys by physical code for letters.** Letter shortcuts (`C`, `O`, `S`, `W`, <kbd>Ctrl</kbd>+`C`,
  `P`, `Z`, `E`) compare `KeyboardEvent.code`, so they work on any keyboard layout. Navigation keys
  compare `key`.
- **Closing.** <kbd>Esc</kbd> closes or hides every window; <kbd>Ctrl</kbd>+<kbd>C</kbd> does the
  same when there is no text selection to copy.
- **⌘ on macOS.** The app's own <kbd>Ctrl</kbd> shortcuts (<kbd>Ctrl</kbd>+<kbd>E</kbd> in the
  input window, <kbd>Ctrl</kbd>+<kbd>Enter</kbd> in the Pad, <kbd>Ctrl</kbd>+<kbd>P</kbd>,
  <kbd>Ctrl</kbd>+<kbd>Z</kbd> in the reader list) check `primary(e)` from `platform.ts`, so on
  macOS they take ⌘ instead, as Mac users expect. <kbd>Ctrl</kbd>+<kbd>C</kbd> still closes there:
  ⌘C copies. Mac keyboards have no forward <kbd>Delete</kbd>, so the reader list takes
  <kbd>⌫</kbd> for <kbd>Del</kbd> there.
- **Keys named as the system does.** Hints, tooltips and the help write keys and words through
  `names` from `platform.ts`: a modifier goes with its key, as `${names.mod}P`, which shows
  `Ctrl+P` on Windows and Linux and `⌘P` on macOS (`⌃C`, `⇧Enter`, `⌫` likewise). Deleted files go
  to the `Recycle Bin` on Windows and the `Trash` elsewhere. Text in static HTML marks the part
  that differs, `<span data-name="mod">Ctrl+</span>P`, and the page calls `nameThings()`. The
  Windows text is the one written in the HTML, so it is what shows unchanged on Windows.
- **Focus stays in the text.** Clickable list items (tag suggestions, calendar days, search
  results) act on `mousedown` with `preventDefault`, so the textarea keeps the focus.
- **Hidden windows refresh on focus.** The input window and the Pad are hidden, not destroyed, so
  they refocus their text and reload tag suggestions on every `focus` event; the reader reloads
  its view on focus.
- **Errors** from commands are shown as `String(err)` in the window (a message line, the status
  bar or an error paragraph) and never only logged.
- **Window sizing.** Pages size their own windows: the input window grows with its text up to ten
  lines, popups fit their content up to a per-window maximum and then scroll, the reminders keep
  their corner after resizing (`place_reminders`).
