# 0017. Watch the notes folder for changes

- **Status:** Accepted
- **Date:** 2026-10-14

## Context

Open windows (the reader, the reminders, the done journal) reload when they receive
`notes-changed`, which the application emits after its own writes. Two kinds of change never
produced it:

- Notes saved through `quicklly add …` or a `quicklly://add` link. By
  [ADR 0007](0007-cli-handled-in-a-fresh-process-before-tauri-starts.md) they are written by a
  separate process that exits before Tauri starts and never contacts the running instance.
- Notes files changed in another editor, such as Obsidian or VS Code, which the plain Markdown
  storage ([ADR 0002](0002-plain-markdown-files-as-the-only-storage.md)) invites.

Open windows showed stale data until their next reload (for the reader, on focus; the reminders
and the done journal not at all). Forwarding requests from the CLI process to the running instance
would cover only the first case and would undo the simplicity of ADR 0007.

## Decision

The running application watches the notes folder recursively with `notify-debouncer-mini`
([`watcher.rs`](../../src-tauri/src/watcher.rs)) and calls the same `notes_changed` that its own
writes call once a change to a notes file has been quiet for 300 ms. A notes file is any `.md`
file in the folder, tag folders and `archive/` included; the top-level `images/` folder,
dot-folders and the temporary `*.md.tmp` files of atomic rewrites do not count (the rename over
the notes file does). The watcher starts in set-up, creates the notes folder if it is missing, and
is replaced when the notes folder changes in settings. Failures are logged and the application
carries on.

The application's own writes are not told apart from other changes.

## Consequences

- Notes saved through the CLI or a link, and edits made in other editors, appear in open windows
  about 300 ms after the last write, whether or not the window has the focus.
- Every write by the application gives two `notes-changed` events: the immediate one and the
  watcher's, about 300 ms later. Windows must handle a repeated reload: the reader skips it while a
  note is being edited, the reminders and the done journal simply reload. The cost is one extra
  read of the files per change. Recognising the application's own writes would need bookkeeping
  of paths and timings that can go wrong; it is not attempted.
- An error shown in a window after a partly failed change (for example, a note deleted but its
  images not moved to the Recycle Bin) is cleared by the watcher's reload.
- The notes folder is created at start rather than with the first note.
- Renaming or moving a whole folder outside the application is reported for the folder only and
  does not reload the windows.
- One more dependency (`notify-debouncer-mini`, with `notify`) and its background threads.
