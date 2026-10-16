# Changelog

All notable changes to Quicklly are listed here, newest first.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and Quicklly
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.3] - 2026-10-16

### Fixed

- A notes file that can't be read, such as one saved by Notepad in another encoding than UTF-8,
  no longer breaks the reader, search, reminders, `/done`, tag suggestions and the archive: it is
  listed with no notes and the rest work without it. Errors about it name the file.
- Deleting or editing a note, or deleting a notes file, no longer shows an error when its images
  can't be moved to the Recycle Bin: the change is saved, Ctrl+Z still undoes it, and the other
  images are still moved.
- Deleting a note in the reader or archiving done notes no longer removes empty hand-written
  headings such as `## Ideas`; only a day heading left with no notes under it goes.
- O in the reader opens only pasted images, right in the `images` folder; a note linking to a
  program or any other file no longer runs it.
- An inline edit in the reader that can't be saved, for example because the file changed in
  another editor meanwhile, stays open with its text and shows why, instead of losing it.
- A pinned Pad or reader no longer opens off-screen after a monitor is disconnected or the
  monitors are rearranged: it keeps its size and opens in its default place instead.
- A Pad draft that can't be read is no longer overwritten: the Pad shows the error instead, and
  Ctrl+E keeps the typed text in the input window with the error.
- Search results show a note's day in the date format of the settings, and no day for a note
  under a heading that is not a date, such as `## Ideas`; such notes are listed after the dated
  ones instead of above them.
- An update download that could not be saved completely (a full disk, the computer turned off
  while saving) is no longer run as the installer at the next start; it is downloaded again.
- Deleting or editing a note in the reader no longer moves an image to the Recycle Bin while
  another note still links to it with the name in other letter case.
- Ctrl+Z in the reader brings back the images of a deleted note when the notes folder is on a
  network share (`\\server\share\…`).
- A hand-made file named like `2026-1-5.md` is no longer taken for a daily file: only
  `YYYY-MM-DD.md` names are, and the reader lists it with the other files by name.
- Tags in scripts with vowel signs, like `#हिंदी` in Devanagari, are highlighted in the Pad and
  suggested while typing as a whole, the same as the file they go to.
- A large notes folder no longer freezes the windows and the hotkey while notes are searched or
  listed: search runs once typing pauses, and each file is read once per search.
- The `/date` calendar in the reader's editor is scrolled into view when it opens, instead of
  being left below the bottom of the window under the last note of a file.
- The reader no longer splits an image label like `[Image #2]` over two lines when a note wraps.
- The block caret at the end of a full line stays on that line, right after its text, instead of
  dropping to the line below; after selecting all and pressing an arrow it no longer stretches the
  reader past its window with scrollbars.

## [0.8.2] - 2026-10-14

### Added

- Snooze in the reminders: S moves a note's date to tomorrow and W to a week from today, so it
  comes back then.
- Tab in the reminders shows the notes ahead, with a later date, by day; `/reminders` opens on
  them when nothing is due.
- `/done` lists the notes done in the last 7 days, archived ones too, by day; C copies them as
  Markdown for a report.
- Ctrl+Z in the reader undoes the deletes, done marks and edits made there, latest first; a
  deleted note comes back with its images.

### Changed

- Open windows (the reader, the reminders and `/done`) update by themselves when a note is saved
  with `quicklly add` or a `quicklly://` link, or when notes files change in another editor such
  as Obsidian or VS Code.
- Search matches notes as they are shown: dates in the date format of the settings
  (`15/10/2026`, not `2026-10-15`), images by their `[Image #N]` label rather than their file
  path, and no longer the hidden done date.

### Fixed

- The help gives the archive delay set in **Archive done notes**, or says the archive is off,
  instead of always "after 7 days".
- `/done` updates while it is open when notes are done, undone, edited or archived elsewhere.
- An image pasted into a note that is then cancelled, or whose label is deleted before saving,
  no longer stays in `images/`: it goes to the Recycle Bin once the input window is cleared or
  the Pad starts over, unless a note, the Pad draft or the input history still has it.
- A task written as `- [ ] HH:MM` keeps its open checkbox when edited in the reader, and a done
  note marked not done again becomes an open task `- [ ]` instead of a plain list item.
- Deleting a note with a hand-written link to something other than a pasted image no longer
  moves that file to the Recycle Bin.
- Update downloads superseded by a newer release before being installed no longer pile up in
  the app cache.

## [0.8.1] - 2026-10-13

### Added

- Type `/date` in a note to pick a day in a calendar, or `/date tomorrow` / `/date 15/10/2026`:
  it is saved as `📅 2026-10-15` like Obsidian Tasks and shown in the new **Date format** setting.
- Reminders: notes with a date of today or earlier pop up in the bottom right corner when
  Quicklly starts and when a new day begins; Space marks one done, Enter opens it, and
  `/reminders` shows them at any time.
- Archive: marking a note done adds the day, `✅ 2026-10-13` like Obsidian Tasks, and done notes
  move to `archive/<file>` after 7 days (see **Archive done notes** in the settings); the reader
  and the search leave the archive out.
- Nested tags: `#work/quicklly` goes to `work/quicklly.md` in a folder, and the reader sorted by
  name shows the folders as a tree.
- Save a note from scripts and links: `quicklly.exe add "buy milk #todo"` prints where it went,
  and `quicklly://add?text=buy%20milk%20%23todo` does the same from a browser, the Run dialog or
  a Stream Deck.

## [0.8.0] - 2026-10-08

### Added

- This changelog: `/changelog` in the input window shows what's new, and each release on
  GitHub takes its notes from it.
- Move a notes file to the Recycle Bin from the reader with Del (press it twice).
- Typing a `#tag` suggests your existing tags, in the input window and in the Pad.
- Links in notes are underlined; the reader shows them short and copies a note's links with C.
- Paste an image with Ctrl+V in the input window or the Pad: it is saved next to the notes as
  `[Image #1]`, and the reader opens a note's images with O.

### Fixed

- The reader shows its header and footer again when you reach either end of a list.
- The cursor goes back to the start when ↓ in the input window's history empties the field.

## [0.7.4] - 2026-09-30

### Added

- Pin the reader with Ctrl+P to keep a file on screen and up to date; a pinned reader can be
  moved and resized and reopens on the same file.
- Sort the reader's files by last change or by name with Tab.

### Changed

- Windows open on the monitor in use instead of always on the main one.
- An unpinned reader closes when you click another window.

### Fixed

- Switching from the input to another window no longer shows both at once.
- Deleting the last note of a day also removes that day's heading.

## [0.7.3] - 2026-09-28

### Added

- `/help` shows the keys, the commands and the version.
- Command answers and errors are shown as text under the note; `/update` says when Quicklly
  is already up to date.

### Changed

- Scrollbars follow the theme.

## [0.7.2] - 2026-09-28

### Fixed

- The reader's layout is no longer broken in the installed app.

## [0.7.1] - 2026-09-28

### Changed

- The reader lists the file written to last first.

### Fixed

- The note text no longer jumps when you start a new line.
- The input window is one line high right after start.

## [0.7.0] - 2026-09-28

### Added

- The Pad for longer notes: open it with → in an empty field or `/pad`, save it as a note with
  Ctrl+Enter.
- Pin the Pad with Ctrl+P to keep it open over other windows, movable and resizable.
- Ctrl+E moves what you typed into the Pad.
- The Pad highlights lines, tags and lists, indents with Tab and continues lists on Enter.
- Edit a reader note in the Pad with Shift+Enter.
- Settings for what the hotkey opens (the quick line or the Pad) and where the Pad goes.
- Transparency and Scale settings for all windows.

### Changed

- Notes keep their blank lines and indentation.
- Windows are 30% see-through by default.

### Fixed

- The input hint hides again while you type.
- Starting Quicklly no longer takes the keyboard from the window in use.

## [0.6.0] - 2026-09-27

### Added

- Search all notes: type `?` in an empty field, then a word.
- A note with a `#tag` goes to the file of that tag, e.g. `buy milk #todo` to `todo.md`.
- Automatic updates: a new version is downloaded in the background and installed on the next
  start, or right away with `/update`; they can be turned off in the settings.

## [0.5.0] - 2026-09-27

### Added

- The notes reader: press ← in an empty field to browse your notes files and the notes in them.
- Mark notes as done in the reader; each file shows its progress and is crossed off when
  every note is done.
- Edit and delete notes in the reader.

## [0.4.0] - 2026-09-27

### Added

- Multi-line notes with Shift+Enter.
- Recall your previous notes with ↑ and ↓ in an empty field.
- Default, Default+ and Light themes.
- Close windows with Ctrl+C, like in a terminal.
- `/exit` quits Quicklly.

### Changed

- A terminal-style block caret and selection.

## [0.3.0] - 2026-09-26

### Added

- Settings: the notes folder, the global hotkey and starting with Windows.
- Single inbox mode: keep all notes in one `inbox.md`, grouped by day, instead of a file per day.
- Open the settings with `/config` in the note input.

### Changed

- The settings are a keyboard-driven, terminal-style popup.
- The note input uses a monospace font.

## [0.2.0] - 2026-09-26

The first release with installers.

### Added

- Windows installers attached to each GitHub release.
- An app icon.

## [0.1.0] - 2026-09-26

The first version, published as source code only.

### Added

- Press Ctrl+Alt+N anywhere, type a note and press Enter: it is added with the time to
  today's Markdown file.
- A tray icon to write a new note, open the notes folder or quit.
- Only one copy of Quicklly runs at a time.

[Unreleased]: https://github.com/ruslan1us/quicklly/compare/v0.8.3...HEAD
[0.8.3]: https://github.com/ruslan1us/quicklly/compare/v0.8.2...v0.8.3
[0.8.2]: https://github.com/ruslan1us/quicklly/compare/v0.8.1...v0.8.2
[0.8.1]: https://github.com/ruslan1us/quicklly/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/ruslan1us/quicklly/compare/v0.7.4...v0.8.0
[0.7.4]: https://github.com/ruslan1us/quicklly/compare/v0.7.3...v0.7.4
[0.7.3]: https://github.com/ruslan1us/quicklly/compare/v0.7.2...v0.7.3
[0.7.2]: https://github.com/ruslan1us/quicklly/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/ruslan1us/quicklly/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/ruslan1us/quicklly/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/ruslan1us/quicklly/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/ruslan1us/quicklly/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/ruslan1us/quicklly/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/ruslan1us/quicklly/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ruslan1us/quicklly/compare/93d807c...v0.2.0
[0.1.0]: https://github.com/ruslan1us/quicklly/tree/93d807c
