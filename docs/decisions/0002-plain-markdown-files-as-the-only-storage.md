# 0002. Plain Markdown files as the only storage

- **Status:** Accepted
- **Date:** 2026-09-26

## Context

Notes are personal data that must outlive the application. Users want to open them in Obsidian or
VS Code, sync them with OneDrive or Git, and edit them by hand. A database or a proprietary format
would lock the data in and make external edits unsafe.

## Decision

Store notes only as UTF-8 Markdown files in a user-chosen folder. Each note is a list item
`- HH:MM text`. The application keeps no database, index or cache of note content: every view
reads the files when it needs them, and every write changes the files directly. Hand-written
content that is not a note is preserved by all operations.

## Consequences

- The user owns the data and can use any tool on it; the format is specified in
  [Data format](../data-format.md) and must stay compatible.
- Reading is done by re-parsing files on demand. This is fast enough for personal note volumes but
  scales with the number and size of files (search reads every file).
- The application must tolerate files edited elsewhere at any time: missing trailing newlines,
  CRLF line endings, unknown lines, concurrent changes. Edits are guarded against stale views
  ([ADR 0010](0010-reader-changes-guarded-by-the-note-as-last-seen.md)).
- Features that would need metadata (reminders, archive dates) must encode it in the text, which
  led to [ADR 0004](0004-obsidian-tasks-formats-for-due-and-done-dates.md).
