# Backend

The backend is the Rust crate in [`src-tauri/`](../src-tauri/) (library `quicklly_lib`). It owns
every file operation, the tray, the global hotkey, window creation and placement, settings,
updates and the background threads. The modules are small and split by responsibility: the note
format logic is pure functions over paths, strings and dates, which keeps it unit-testable without
a running application, while thin `#[tauri::command]` functions in [`lib.rs`](../src-tauri/src/lib.rs)
and the window modules connect it to the pages. This page lists the modules, then the conventions
for errors, file writes and undo.

## Modules

| Module | Responsibility |
| --- | --- |
| [`lib.rs`](../src-tauri/src/lib.rs) | Entry point `run()`: CLI short-circuit, plugins, managed state, set-up, window-event rules, tray, the command list. Note commands that combine modules (save, edit, delete, undo, search, done journal), popup creation (`show_popup`), scale and acrylic effect helpers, `notes_changed`. |
| [`notes.rs`](../src-tauri/src/notes.rs) | Writing new notes: normalisation (`note_lines`, `format_note`), tag detection (`first_tag`), target file choice, headings, append (`append_note`). The `Mode` enum (daily or inbox). |
| [`reader.rs`](../src-tauri/src/reader.rs) | Reading and changing existing notes: listing files as a tree, parsing, search, due and done queries, done marks, snooze, edit, delete, undo, file deletion, path validation, line-ending-preserving atomic rewrites. |
| [`archive.rs`](../src-tauri/src/archive.rs) | Moving done notes to `archive/<same path>` after the configured number of days. |
| [`images.rs`](../src-tauri/src/images.rs) | Saving pasted images, relinking image paths for nested files, resolving paths safely, finding images no note links to, moving them to the Recycle Bin and restoring them. |
| [`settings.rs`](../src-tauri/src/settings.rs) | `settings.json` keys, typed getters with defaults, the settings view and all setting commands, hotkey registration and change, appearance and date-format events. |
| [`hotkey.rs`](../src-tauri/src/hotkey.rs) | Default hotkey, parsing from the `KeyboardEvent.code` form, human-readable labels. |
| [`pad.rs`](../src-tauri/src/pad.rs) | The Pad window: creation, placement, pin state, bounds, draft file, moving input text into the draft, editing a reader note. |
| [`reader_window.rs`](../src-tauri/src/reader_window.rs) | The reader window's pin state, pinned file, saved bounds and sort order. |
| [`window_bounds.rs`](../src-tauri/src/window_bounds.rs) | Saving and restoring a pinned window's position and size; minimum size and resizability. |
| [`monitor.rs`](../src-tauri/src/monitor.rs) | The monitor to open on (foreground window, then mouse, then primary) and centring on it. Uses `GetForegroundWindow` and `GetWindowRect` on Windows. |
| [`reminders.rs`](../src-tauri/src/reminders.rs) | The daily-round thread (archive, then reminders), the reminders window and its commands. |
| [`updater.rs`](../src-tauri/src/updater.rs) | Background update checks, download to the cache, install at the next start or on `/update`. |
| [`cli.rs`](../src-tauri/src/cli.rs) | `quicklly add …` and `quicklly://add?text=…`: request parsing, saving without Tauri, console output. |

## Module notes

### `notes`

`append_note(dir, mode, now, text)` is the single writer of new notes; the input window, the Pad
and the CLI all reach it. It returns the path written to, or `None` for a blank note. It reads the
existing file only to decide on the title, the day heading and a missing trailing newline, then
opens the file in append mode. The full format is in [Data format](data-format.md#writing-a-new-note).

### `reader`

- `list_files` walks the notes folder recursively (skipping `archive/`, `images/` and dot-folders),
  counts notes and done notes per file and returns modification times in milliseconds. The order
  is a tree: in each folder `inbox.md`, then other files by name, then daily files newest first,
  then subfolders by name. The reader re-sorts by modification time for its "recent first" view.
- `parse` is the shared parser used by listing, search, due and done queries, edits and the
  archive.
- `search(dir, query, limit)` is a case-insensitive substring match over each note's stored text;
  the day of a note is the daily file's date or the last `## ` heading above it; results are
  sorted newest first by day and time and truncated to `limit` (50 from the command).
- `due(dir, today, ahead)` returns not-done notes whose first due date is on or before today, or
  after today when `ahead`, oldest date first; equal dates keep file and line order (stable sort).
  `overdue` is the number of days since the date (negative ahead).
- `done_since(dir, from)` scans the notes folder and `archive/`, returns notes whose done date is on
  or after `from`, latest day first, without the done date in the text. Archived files are named
  `archive/…`.
- Changes to existing notes go through `edit_note`; see [below](#changes-to-existing-notes).
- `trash_file` moves a notes file to the Recycle Bin and then removes the folders it was in while
  they are empty.

### `archive`

`archive_all(dir, today, days)` processes every notes file. For each file, the done notes whose
done date is at least `days` days before today are appended to `archive/<same path>` under their
original `## YYYY-MM-DD` heading (a new heading only when the archive's last heading differs), with
image links adjusted by one `../`. The archive file is written first and the notes file second: if
the second write fails the notes exist twice rather than not at all. The notes file is then
rewritten without them, and day headings left with nothing under them are removed
(`remove_empty_days`; a heading with hand-written text under it stays). A new archive file starts
with the source file's `# ` title, or `# <name>` when it has none.

`archive_done(app)` runs it with the setting and the local date; it is called by the daily round
and by `set_archive_after`, and emits `notes-changed` when anything moved. See
[ADR 0005](decisions/0005-archive-done-notes-per-file-after-n-days.md).

### `images`

- `save` writes the bytes under a time-based name (see [Data format](data-format.md#images)).
- `link_from(text, name)` rewrites `](images/` to `](../images/` once per folder level of the
  notes file; links already relative are untouched.
- `resolve(dir, file, path)` turns a link in a notes file into a path inside the notes folder,
  component by component, and returns `None` for anything that could leave the folder (absolute
  paths, drives, `.`, too many `..`). `open_images` uses it before opening anything.
- `trash_unused(dir, file, paths)` moves to the Recycle Bin the images a deleted or edited note
  linked to, unless any notes file (archive included, links resolved from each file's folder)
  still links to them; it returns their original full paths. `restore(paths)` brings each back by
  finding the latest Recycle Bin entry with that original path, and returns how many could not be
  restored.

### `settings`

Getters (`mode`, `hotkey_target`, `pad_position`, `transparency`, `scale`, `archive_after`,
`auto_update`, …) read the store on each call and fall back to the default on a missing or invalid
value. Setters are commands that write the store, apply side effects (re-register the hotkey,
re-zoom windows, archive, start or stop update checks, emit events) and return the complete
`SettingsView`, which the settings page renders as is. `notes_settings_in(file)` reads the notes
folder and mode straight from `settings.json` for the CLI, which runs without the store plugin.
Enum values are serialised with the exact strings the frontend uses; tests pin them.

### `pad` and `reader_window`

Both keep a pin flag in managed state (an `AtomicBool`, loaded at start) so that the window-event
handler can decide quickly whether to hide, close or remember bounds. Pinned windows are resizable
with a minimum size (Pad 360×200, reader 440×200 logical pixels at 100 % scale) and save their
bounds in physical pixels on every move or resize. The Pad's draft is a plain file,
`pad-draft.md`, removed when empty.

### `reminders`

`start` runs the daily round in a thread: on start and whenever the local date changes, it calls
`archive::archive_done` and, if any note is due, shows the reminders without focus. A reminders
window that is already open is only sent `notes-changed`. `open_reminders` (the `/reminders`
command) returns `false` and opens nothing when there is nothing due and nothing ahead.

### `updater`

A thread checks at start and then every 6 hours while automatic updates are on. A found release is
downloaded (the plugin verifies the signature) and stored as `update-<version>.bin` in the app
cache directory; at the next start the stored bytes are installed instead of downloaded again.
`install_update` (`/update`) installs right away, from the cache when possible. Turning automatic
updates off clears the announced version; turning them on checks immediately.

### `cli`

Described in [CLI and links](cli-and-links.md).

## Conventions

### Errors

- Commands return `Result<T, String>`. Errors are mapped at the command boundary with context that
  a user can act on, for example `Failed to save note in <folder>: <io error>` or
  `Couldn't update <file>: <reason>`. Pages show the string as is.
- Inner functions return `io::Result` (or `tauri::Result`) and use `?`.
- Background work (update checks, the daily round, hotkey registration at start, opening the notes
  folder from the tray) reports failures with `eprintln!` and carries on; it never panics or stops
  the application.
- Window operations whose failure has no useful remedy (`show`, `set_focus`, `set_position`, …)
  are deliberately ignored with `let _ =`.
- Settings getters never fail: a missing or unreadable value means the default.

### File writes

| Operation | Method | Why |
| --- | --- | --- |
| New note, archive entry | Append (`OpenOptions::append`) | Never rewrites existing content; a crash can at most leave a partial last entry. |
| Edit, done mark, snooze, delete, undo, archive source rewrite | `reader::write_file`: write `<file>.md.tmp`, then rename over the file | A failed write cannot leave a half-written notes file. |
| Pasted image | `create_new` under a unique name | Never overwrites an existing image. |
| Pad draft | `fs::write` | Disposable working copy, rewritten every few hundred milliseconds while typing. |
| Deleting notes files and images | `trash::delete` (Recycle Bin) | Recoverable outside the application; see [ADR 0015](decisions/0015-deletions-go-to-the-recycle-bin.md). |

### Changes to existing notes

Every change to an existing note goes through `reader::edit_note`. The page sends the note as it
last saw it (`NoteRef`: line, time, done flag, full text). `edit_note` re-reads and re-parses the
file and refuses the change, without writing, unless a note with exactly those values still starts
at that line ("the file has changed since it was shown, so it has been reloaded"). Only the lines
of that note are replaced; everything else in the file is kept byte for byte apart from the
rewrite of line endings described in [Data format](data-format.md#line-endings). See
[ADR 0010](decisions/0010-reader-changes-guarded-by-the-note-as-last-seen.md).

### Undo snapshots

`edit_note` returns a `Change { before, after, images }`: the whole file content before and after
the change, and (filled in by the command) the original paths of images moved to the Recycle Bin.
The reader keeps these in memory. `undo` writes `before` back only if the file content is still
exactly `after`; otherwise it fails with "<file> has changed since" and nothing is written. After a
successful file restore, `images::restore` brings the images back. Snapshots of a whole file are
used rather than inverse operations because they restore the exact bytes, including removed day
headings and hand-written text, with no per-operation logic. See
[ADR 0011](decisions/0011-undo-via-whole-file-snapshots.md).

### Notifying windows

A command that changes a notes file calls `notes_changed(&app)` after the write, which emits
`notes-changed` to the reader and the reminders. New commands that write notes must do the same.

### Code style

Code passes `cargo fmt` and `cargo clippy --all-targets -- -D warnings`. Public items and most
private functions have doc comments that explain behaviour and reasons in plain sentences; the
existing comments use British spelling (`colour`, `centre`). Unsafe code is limited to the read-only
Win32 window calls in `monitor.rs` (with a `SAFETY` comment) and the console calls in `cli.rs`.
