# 0003. A tag is a file; nested tags are folders

- **Status:** Accepted
- **Date:** 2026-09-27 (nested tags added 2026-10-13)

## Context

Notes need lightweight organisation without leaving the input window and without settings.
Commands such as `/todo` were considered and rejected: `/` commands are reserved for controlling
the application. In a plain-file store ([ADR 0002](0002-plain-markdown-files-as-the-only-storage.md)),
the most direct way to group notes is to put them in the same file.

## Decision

A note whose text contains a `#tag` is appended to `<tag>.md` instead of the daily file or the
inbox, grouped by day like the inbox. The first tag decides; the tag stays in the text; the file
name is the tag in lower case. There is no configuration of tag files.

Nested tags (`#work/quicklly`) map to folders (`work/quicklly.md`), at any depth, created on
demand. A nested tag whose first part is `archive` or `images` is reduced to that part so that tags
can never write into the application's reserved folders.

## Consequences

- Tag files are ordinary notes files: they appear in the reader, search and reminders, and Obsidian
  shows them as normal files and folders.
- A note lives in exactly one file. Notes with several tags are not duplicated; only the first tag
  routes them.
- The tag grammar is fixed (`#` at a word start, then a letter), so `C#`, `#1` and dates are not
  tags. The same grammar is implemented in `notes::first_tag`, the tag suggestions and the Pad's
  highlighting, and must be changed in all of them together.
- Image links in nested files are relative to the file's folder (`../images/…`), which requires
  relinking when notes are written, edited or archived.
