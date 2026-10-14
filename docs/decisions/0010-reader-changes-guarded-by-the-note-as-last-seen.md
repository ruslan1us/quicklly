# 0010. Reader changes guarded by the note as last seen

- **Status:** Accepted
- **Date:** 2026-09-27

## Context

The reader changes existing notes (done marks, edits, deletes, and later snoozing from the
reminders). The files can change at any time behind it: a note saved from the input window or the
CLI, a hand edit, the archive. Addressing a note by line number alone could change the wrong note;
rewriting the whole file from the reader's copy could discard other changes.

## Decision

Each change carries the note as the page last displayed it (line, time, done flag, full text). The
backend re-reads and re-parses the file and applies the change only if exactly that note still
starts at that line. Only that note's lines are replaced; the rest of the file, including
unrecognised content and the file's line endings, is kept. The write goes through a temporary file
and a rename. If the check fails, nothing is written and the reader reloads the file and shows the
error.

## Consequences

- A stale view can never change the wrong note or overwrite newer content.
- Users occasionally see "the file has changed since it was shown" and must repeat the action
  after the reload.
- The check is optimistic and not a lock: a write between the check and the rename is still
  possible, though the window is very short.
- The returned before/after content made undo straightforward
  ([ADR 0011](0011-undo-via-whole-file-snapshots.md)).
