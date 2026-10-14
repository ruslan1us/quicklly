# 0011. Undo in the reader via whole-file snapshots

- **Status:** Accepted
- **Date:** 2026-10-14

## Context

Deletes, done marks and edits in the reader are single key presses and can be made by mistake.
Inverse operations per change type (re-insert a deleted note, toggle back, restore old text) would
need to recreate removed day headings, done dates and exact formatting, and would each need their
own edge cases. Deleted images are in the Recycle Bin.

## Decision

Every reader change returns a `Change` with the complete file content before and after it, plus
the original paths of images it moved to the Recycle Bin. The reader keeps a stack of these in
memory while it is open. <kbd>Ctrl</kbd>+<kbd>Z</kbd> writes the `before` content back only if the
file is still exactly the `after` content, then restores the images from the Recycle Bin. The stack
is dropped when the reader closes.

## Consequences

- Undo restores the exact bytes, including headings and hand-written text, with one generic
  mechanism.
- Undo is refused once anything else has changed the file (a new note, the archive at a day
  change, a hand edit); the entry is then dropped.
- Snapshots of whole files are held in memory for the session; acceptable for personal note files.
- File deletions, Pad edits and reminder actions are not undoable this way.
- Image restore depends on `trash::os_limited`, which is not available on macOS.
