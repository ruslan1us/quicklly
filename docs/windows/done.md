# Done journal

The done journal (`/done`) lists the notes marked done in the last seven days, today included,
from every notes file and the archive, grouped under the day each was done, latest day first. It
exists for weekly reports and retrospectives: <kbd>C</kbd> copies the list as Markdown. It is a
read-only view. User-facing description: root README, [Done](../../README.md#done).

## Files

| File | Role |
| --- | --- |
| [`src/windows/done/index.html`](../../src/windows/done/index.html) | Title, rows, empty and error lines, footer |
| [`src/windows/done/done.ts`](../../src/windows/done/done.ts) | Behaviour |
| [`src/windows/done/done.css`](../../src/windows/done/done.css) | Layout; imports `terminal.css`; done notes are not struck through here |
| [`src-tauri/src/lib.rs`](../../src-tauri/src/lib.rs) | `open_done`, `list_done` (`DONE_DAYS = 7`) |
| [`src-tauri/src/reader.rs`](../../src-tauri/src/reader.rs) | `done_since` |

## Behaviour

- `list_done` computes the first day as today minus six days and calls `reader::done_since`, which
  reads every notes file and every file in `archive/` and keeps done notes whose done date
  (`✅ YYYY-MM-DD` at the end of the first line) is on or after that day.
- Notes are sorted by done date, latest first, with a stable sort: within a day, notes keep the
  order of the files (notes folder before archive) and of the lines.
- Each row shows the first line of the note, with links shortened, images as labels and dates in
  the date format. Each day starts with a `── <date>` row in the date format.
- The window fits its content up to 400 CSS pixels and then scrolls; it is shown with `reveal()`.
- It reloads on `notes-changed` (a note done or undone in the reader or the reminders, or the
  archive), refits itself and keeps its scroll position as far as the new list reaches. A message
  in the footer stays.

## Keys

| Key | Action |
| --- | --- |
| <kbd>↑</kbd> <kbd>↓</kbd> | Scroll by one step |
| <kbd>PgUp</kbd> <kbd>PgDn</kbd> | Scroll by a page, keeping one step of overlap |
| <kbd>C</kbd> | Copy as Markdown |
| <kbd>Esc</kbd>, <kbd>Ctrl</kbd>+<kbd>C</kbd> | Close |

## Copied Markdown

```markdown
## 13/10/2026
- pay rent 📅 15/10/2026
- groceries:
  milk
## 12/10/2026
- call the bank
```

The copy is meant to be pasted into a report, so it uses the display form: headings and dates in
the date format, images as `[Image #N]` labels, and the full text of multi-line notes with
continuation lines indented by two spaces.

## Limitations

- Notes marked done before version 0.8.1 have no done date and are not listed.
- The seven-day window is fixed in code (`DONE_DAYS`), not a setting.
