# Data format

All notes are plain UTF-8 Markdown files in one folder, the notes folder; the application keeps no
other copy of them. This page specifies the folder layout, the note line format that every reader
and writer in the code shares, tags, dates, images and the conversions between how text is shown
in a window and how it is stored, the reserved names, and the application's own files outside the
notes folder. Code that reads or writes notes must keep to this format: the files are also edited
by hand and read by other tools such as Obsidian.

The user-facing description is in the root README, [Where notes are stored](../README.md#where-notes-are-stored).

## The notes folder

The notes folder is `Documents\Quicklly` unless another folder is chosen in the settings
(`settings::notes_dir`). It and its subfolders are created on the first write.

```text
Quicklly/                      notes folder
  2026-10-13.md                daily file (mode "daily", untagged notes)
  inbox.md                     inbox (mode "inbox", untagged notes)
  todo.md                      tag file for #todo
  work/
    quicklly.md                tag file for #work/quicklly
  My Notes.md                  any other Markdown file: listed and editable, never written to by tags
  images/
    2026-10-13_14-32-05.png    pasted image
  archive/
    todo.md                    done notes moved out of todo.md
    work/
      quicklly.md
  .obsidian/                   dot-folders are ignored
```

| File | Written by | Title line | Day headings |
| --- | --- | --- | --- |
| `YYYY-MM-DD.md` (daily) | Untagged notes in daily mode | `# YYYY-MM-DD` | None |
| `inbox.md` | Untagged notes in inbox mode | `# Inbox` | `## YYYY-MM-DD` |
| `<tag>.md`, `<a>/<b>.md` | Notes whose first tag is `#tag` or `#a/b` | `# tag`, `# a/b` | `## YYYY-MM-DD` |
| `archive/<path>` | The archive, for the notes file at `<path>` | Title of the source file | As in the source file |
| Any other `.md` | The user | Not required | Optional |

**Notes files** are all `.md` files in the notes folder and its subfolders, except anything under
the top-level `archive/` and `images/` folders (matched case-insensitively) and any path with a
part starting with `.`. Names travel between Rust and the pages as paths relative to the notes
folder with `/` separators, for example `work/quicklly.md`. Every name received from a page is
validated by `reader::notes_file` / `is_notes_file` in [`reader.rs`](../src-tauri/src/reader.rs):
no `..`, no absolute paths or drives, no `\`, no `:`, and it must end in `.md`. Nothing outside the
notes folder can be read, changed or deleted through a command.

Files are read as UTF-8 only; no other encoding is guessed and nothing is read lossily. A notes
file that can't be read (not UTF-8, as Notepad may save it, or locked) does not stop the rest:

| Where | What happens to that file |
| --- | --- |
| File list (`reader::list_files`), tag suggestions | Listed with no notes |
| Opening, editing or saving a note to it | The error names the file |
| Search, reminders, done journal | Left out |
| Archive | Left as it is, and so is its archive file; an archive file that can't be read leaves its notes file as it is too |
| Image clean-up (`images::unused`, `images::unsaved`) | Fails, naming the file, so an image it may link to is never moved to the Recycle Bin |

## Note lines

A note is one Markdown list item:

```markdown
- 14:32 first line of the note
  second line, indented by two spaces

  a blank line inside the note is kept
- [x] 15:10 a done note ✅ 2026-10-13
- 16:00 call the bank 📅 2026-10-15 #todo
```

| Element | Rule |
| --- | --- |
| First line | `- `, optionally a checkbox `[x] ` or `[X] ` (done) or `[ ] ` (not done), then the time `HH:MM` (two digits, colon, two digits), then a space and the text, or nothing (a note may be empty after the time). |
| Continuation lines | Lines indented by exactly two spaces that directly follow the note. Blank lines belong to the note only when another indented, non-blank line follows them. |
| Everything else | Headings, paragraphs, other list items and hand-written content are not notes. They are ignored by the parser and preserved by every edit. |

The parser is `reader::parse`; it returns day headings (`## …` lines) and notes, each note with
the 0-based index of its first line, its time, its done flag and its text (continuation lines
joined with `\n`, without the two-space indentation).

### Writing a new note

`notes::append_note` in [`notes.rs`](../src-tauri/src/notes.rs) normalises the text before writing:

1. Trailing whitespace is removed from every line, blank lines before and after the text are
   dropped, and the first line loses its leading whitespace (`note_lines`).
2. Further lines are indented by two spaces; blank lines stay empty; indentation inside the note
   is kept (`format_note`).
3. A note that is blank after this is not written (the input window simply closes).

The entry is appended to the end of the file. A missing trailing newline (from a hand edit) is
added first. A new file starts with its title line and a blank line. In a grouped file (inbox, tag
file) a `## YYYY-MM-DD` heading is written before the note when the last `## ` heading in the file
is not today's date, preceded by a blank line unless the file is new. The time is the local time
of saving.

```markdown
# Inbox

## 2026-09-26

- 14:32 check why swap keeps growing on prod3

## 2026-09-27

- 09:05 book a dentist appointment
```

### Line endings

New files are written with `\n`. Edits made in the reader, the archive and undo keep the file's
line endings: Windows line endings if the file contains any `\r\n`, otherwise `\n`, and the final
newline only if the file had one (`reader::newline_of`, `reader::write_lines`).

## Tags

A tag is `#` at the start of a word (after whitespace or at the start of the text), followed by a
letter, then letters, digits, `_` or `-`. `#1`, `#2026-09-26`, `C#` and `a#b` are not tags. A
nested tag adds parts after `/`, each starting with a letter: `#work/quicklly`. A part that does
not start with a letter ends the tag, so `#work/` and `#work//x` are `work`.

- Only the **first** tag in the note decides the file (`notes::first_tag`). The tag stays in the
  note text.
- The file name is the tag in lower case: `#Todo` goes to `todo.md`, `#Work/Quicklly` to
  `work/quicklly.md`. Folders are created as needed, at any depth.
- A nested tag whose first part is `archive` or `images` is reduced to that first part, so
  `#archive/x` goes to `archive.md` and never into the reserved folders.
- The tags suggested while typing (`reader::list_tags`) are the names of notes files that a tag
  would produce: not the inbox, not daily files, and only names that round-trip through
  `first_tag` (so `My Notes.md` is not offered).

See [ADR 0003](decisions/0003-a-tag-is-a-file-nested-tags-are-folders.md).

## Dates

Dates follow the [Obsidian Tasks](https://publish.obsidian.md/tasks/) emoji format and are always
stored in ISO form. See [ADR 0004](decisions/0004-obsidian-tasks-formats-for-due-and-done-dates.md).

| Mark | Meaning | Stored as | Where |
| --- | --- | --- | --- |
| `📅` | Due date | `📅 YYYY-MM-DD` | Anywhere in the note, typed with `/date` |
| `✅` | Done date | `✅ YYYY-MM-DD` | At the end of the first line of a done note |

- **Due date.** The first `📅 ` followed by a real date in exactly `YYYY-MM-DD` form, not followed
  by another digit, is the note's date for reminders and snoozing (`reader::first_date`). Further
  dates in the note are ordinary text. Snoozing replaces only that first date.
- **Done date.** Marking a note done writes `- [x] HH:MM text ✅ YYYY-MM-DD` with the current date;
  a note with an empty text becomes `- [x] HH:MM ✅ YYYY-MM-DD`. Marking it not done writes an
  open checkbox, `- [ ] HH:MM text`, and removes a trailing done date. Only a `✅ date` at the very
  end of the first line, after a space or alone, counts (`reader::split_done_date`); one in the
  middle of the text is left alone. The pages hide the done date, and an edit keeps it and the
  checkbox (or its absence) as they are.
- **Open checkbox.** New notes are written as `- HH:MM text`, without a checkbox. A note that was
  done and is marked not done again becomes an open task, `- [ ] HH:MM text`, as Obsidian would
  have it; a done line does not record whether it had `[ ]` before, so the plain form is not
  restored. Quicklly treats `- [ ] HH:MM` and `- HH:MM` the same everywhere.
- Notes done without a done date (marked before version 0.8.1) are never archived and do not
  appear in the done journal.
- **Display.** Windows show and accept dates in the **Date format** setting (`DD/MM/YYYY`,
  `MM/DD/YYYY`, `YYYY-MM-DD`, `DD.MM.YYYY`); [`dates.ts`](../src/shared/dates.ts) converts with
  `toDisplay` and `toStored`. A shown date that is not a real calendar date, or that is part of a
  longer number, is left as typed text.

## Images

A pasted image is saved by `images::save` in [`images.rs`](../src-tauri/src/images.rs) as
`images/YYYY-MM-DD_HH-MM-SS.<ext>` in the notes folder, at the moment of pasting. The extension
comes from the MIME type (`jpg`, `gif`, `webp`, `bmp`, otherwise `png`). A second image in the same
second gets `_2`, then `_3`; files are created with `create_new`, so an existing file is never
overwritten.

In a note the image is a Markdown image whose label is `Image #N` and whose path is relative to
the folder of the notes file, as Obsidian resolves it:

| Notes file | Stored link |
| --- | --- |
| `todo.md` | `![Image #1](images/2026-10-07_14-32-05.png)` |
| `work/quicklly.md` | `![Image #1](../images/2026-10-07_14-32-05.png)` |
| `archive/work/quicklly.md` | `![Image #1](../../images/2026-10-07_14-32-05.png)` |

In text fields the image appears only as its label, `[Image #1]`; the page keeps a map from label
number to path. See [ADR 0006](decisions/0006-pasted-images-saved-as-files-next-to-the-notes.md).

<kbd>O</kbd> in the reader opens a link only when it points to a file right in the top-level
`images/` folder with an extension pasted images are saved with; a hand-written link to anything
else, such as a program, is never opened.

Images leave `images/` only to the Recycle Bin, in two cases:

| When | Which images | Kept while |
| --- | --- | --- |
| A note is deleted or edited in the reader, or a notes file is deleted | The images the note or file linked to | Any notes file, archive included, links to the same file (links resolved from each file's folder, ignoring case) |
| The input window's note is cleared, or the Pad saves a note or loads the draft or a note to edit in its place | The images that window held: pasted into it, and in the Pad also those of the draft when it was loaded | Any notes file, archive included, the Pad draft or the input window's history has the image's file name in it, ignoring case |

The second case only ever considers files right in the top-level `images/` folder. See
[ADR 0015](decisions/0015-deletions-go-to-the-recycle-bin.md) and
[ADR 0018](decisions/0018-pasted-images-let-go-of-when-the-text-starts-over.md).

## Display and stored forms

Text has two forms. The **stored** form is what files contain: images as Markdown links, dates as
`📅 YYYY-MM-DD`. The **display** form is what a text field shows: images as `[Image #N]` labels,
dates in the date format. Every boundary converts explicitly:

| Boundary | Conversion | Code |
| --- | --- | --- |
| Input window or Pad saves a note | `toStored(toMarkdown(text, images))`; Rust then links images from the target file's folder (`images::link_from`) | `main.ts` `stored()`, `pad.ts` `stored()`, `notes::append_note` |
| Pad draft saved or loaded | Draft stored in stored form; loaded with `fromMarkdown(toDisplay(draft))` | `pad.ts` `saveDraft`, `setText` |
| Input history recalled | `fromMarkdown(toDisplay(entry))` | `main.ts` `showHistory` |
| Reader, reminders, done list, search results show a note | `toDisplay` for dates, `markdownNodes` for image labels, `shortLink` for links | each window's `noteText` |
| Reader or Pad edits a note | `fromMarkdown(toDisplay(text without done date))` in; `toStored(toMarkdown(…))` out; Rust re-links new `images/…` paths and keeps the done date | `reader.ts` `startEdit`/`saveEdit`, `reader::set_text` |
| Archive moves a note | Image links gain one `../` for the `archive/` folder | `archive::archived_line` |
| Done journal copied | Display form: `## <shown date>` headings, `- text` with image labels and shown dates | `done.ts` `markdown()` |
| Date format changed | Open text fields re-render: `toDisplay(toStored(text, previous))` | `followDateFormat` callbacks |
| Search matches a note | Rust builds the display form: done date dropped, `images::labels` for image labels, dates in the date format (`DateFormat::format`) | `reader::shown_text` |

Search matches what the results show (`reader::search`): a date is found in the date format, an
image by its label `[Image #N]` and not its path, and the hidden done date not at all. Links are
the exception: they are matched in full, while the results show them short.

## Reserved names

| Name | Reserved for |
| --- | --- |
| `inbox.md` | The inbox; first in its folder in the reader's by-name order |
| `YYYY-MM-DD.md` (a real date written exactly so, in any folder; `2026-1-5.md` is not one) | Daily files; after the other files of their folder in by-name order, newest first; never tag files |
| `archive/` (top level) | The archive; excluded from listings, search, reminders and tags |
| `images/` (top level) | Pasted images; excluded from listings |
| Names starting with `.` | Ignored at every level (for example `.obsidian/`, `.git/`) |
| `*.md.tmp` | Temporary file of an atomic rewrite, renamed over the original |

## Application files

Outside the notes folder the application keeps these files in its data folder,
`%APPDATA%\com.quicklly.app` (the identifier from `tauri.conf.json`):

| File | Content |
| --- | --- |
| `settings.json` | Settings, written by `tauri-plugin-store` |
| `pad-draft.md` | The Pad's unsaved draft, in stored form; deleted when the draft is empty |

Downloaded updates wait in the app cache directory as `update-<version>.bin` until installed,
written first as `update-<version>.bin.tmp` and renamed into place. The next update check removes
any `update-<version>.bin` other than the one for the latest release.

### `settings.json` keys

A missing key means the default. Values that equal the default are stored by some setters and
deleted by others; both read the same.

| Key | Values | Default | Set by |
| --- | --- | --- | --- |
| `notesDir` | Absolute path; absent or empty for the default | `Documents\Quicklly` | `pick_notes_dir`, `reset_notes_dir` |
| `mode` | `"daily"`, `"inbox"` | `"daily"` | `set_mode` |
| `hotkey` | Global-shortcut string, for example `"control+alt+KeyN"`; absent for the default | <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>N</kbd> | `set_hotkey`, `reset_hotkey` |
| `hotkeyOpens` | `"input"`, `"pad"` | `"input"` | `set_hotkey_target` |
| `padPosition` | `"topRight"`, `"topLeft"`, `"bottomRight"`, `"bottomLeft"`, `"center"` | `"topRight"` | `set_pad_position` |
| `dateFormat` | `"DD/MM/YYYY"`, `"MM/DD/YYYY"`, `"YYYY-MM-DD"`, `"DD.MM.YYYY"` | `"DD/MM/YYYY"` | `set_date_format` |
| `archiveAfter` | `"never"`, `"oneDay"`, `"threeDays"`, `"sevenDays"`, `"fourteenDays"`, `"thirtyDays"` | `"sevenDays"` | `set_archive_after` |
| `theme` | `"default"`, `"defaultPlus"`, `"light"` | `"default"` | `set_theme` |
| `transparency` | Percent, clamped to 0–90 (the settings window offers 0–80) | `30` | `set_transparency` |
| `scale` | Percent, clamped to 50–200 (the settings window offers 80–150) | `100` | `set_scale` |
| `autoUpdate` | `false`; absent when on | on | `set_auto_update` |
| `padPinned` | Boolean | `false` | `set_pad_pinned` |
| `padBounds` | `{ x, y, width, height }` in physical pixels | none | Pad moved or resized while pinned |
| `readerPinned` | Boolean | `false` | `set_reader_pin` |
| `readerFile` | Notes file name shown by the pinned reader | none | `set_reader_pin` |
| `readerBounds` | `{ x, y, width, height }` in physical pixels | none | Reader moved or resized while pinned |
| `readerSort` | `"recent"`, `"name"` | `"recent"` | `set_reader_sort` |

Start with Windows is not stored here: it is a `Run` entry in the user registry managed by
`tauri-plugin-autostart`. Invalid values fall back to the default (`settings::stored`).
