# 0005. Archive done notes per file after N days

- **Status:** Accepted
- **Date:** 2026-10-13

## Context

Tag files and the inbox grow without bound, and completed notes crowd out open ones in the reader,
search and reminders. Deleting completed notes would lose history; moving them to one global
archive file would lose their grouping.

## Decision

A done note moves, a configurable number of days after its done date (never, 1, 3, 7 by default,
14 or 30), from its notes file to the file at the same relative path under `archive/`
(`todo.md` → `archive/todo.md`, `work/quicklly.md` → `archive/work/quicklly.md`), under its
original day heading. Archiving runs at start, at each change of the local date, and when the
setting changes. The archive file is written before the notes file is rewritten.

## Consequences

- Working files stay short; history keeps its structure and stays readable in Obsidian.
- The reader, search, reminders and tag suggestions exclude `archive/`; the done journal and the
  unused-image check include it.
- Image links in archived notes gain one `../`.
- If rewriting the notes file fails after the archive was written, a note can exist in both files;
  it is never lost.
- Archiving changes files at the day change, which invalidates pending reader undo entries for
  those files ([ADR 0011](0011-undo-via-whole-file-snapshots.md)).
- Notes done without a done date are never archived.
