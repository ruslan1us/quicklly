# 0015. Deletions go to the Recycle Bin

- **Status:** Accepted
- **Date:** 2026-10-07

## Context

The reader can delete whole notes files, and deleting or editing notes can leave pasted images
unreferenced. Permanent deletion of user files from a keyboard-driven tool is risky.

## Decision

Notes files deleted from the reader and images no longer linked from any note are moved to the
Windows Recycle Bin with the `trash` crate, never deleted permanently. Folders left empty by a
deleted notes file are removed. Deleting a note within a file is not a file deletion; it is
protected by the reader's undo ([ADR 0011](0011-undo-via-whole-file-snapshots.md)).

## Consequences

- A mistaken deletion can be recovered from the Recycle Bin outside the application, and undo can
  bring images back automatically.
- Deleting a file requires a second confirming key press in the reader.
- Image restore uses `trash::os_limited`, which is Windows- and Linux-specific.
- Tests cannot exercise the real Recycle Bin; they test the decisions around it
  ([Testing](../testing.md#rules)).
