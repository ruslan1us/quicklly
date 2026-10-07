# Changelog

All notable changes to Quicklly are listed here, newest first.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and Quicklly
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Move a notes file to the Recycle Bin from the reader with Del (press it twice).
- Typing a `#tag` suggests your existing tags, in the input window and in the Pad.

### Fixed

- The reader shows its header and footer again when you reach either end of a list.

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

[Unreleased]: https://github.com/ruslan1us/quicklly/compare/v0.7.4...HEAD
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
