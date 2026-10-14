# 0006. Pasted images saved as files next to the notes, shown as labels

- **Status:** Accepted
- **Date:** 2026-10-08

## Context

Screenshots are a common part of quick notes. The input window is a plain text field that cannot
display images, and the storage is Markdown files
([ADR 0002](0002-plain-markdown-files-as-the-only-storage.md)). Embedding image data in the text
(base64) would make files unreadable.

## Decision

On paste, the image is saved immediately as a file in `images/` in the notes folder, named by the
time of pasting (`YYYY-MM-DD_HH-MM-SS.<ext>`, numbered within the same second, never overwriting).
The text field shows a short label, `[Image #N]`; the page keeps the number-to-path map. When the
text leaves the field, labels become Markdown image links, `![Image #N](images/….png)`, relative to
the folder of the notes file so that Obsidian renders them. Images that no note links to any more
after a delete or an edit go to the Recycle Bin.

## Consequences

- Notes stay readable as text; images are ordinary files the user owns.
- Text has a display form and a stored form, converted at every boundary
  ([Data format](../data-format.md#display-and-stored-forms)).
- Links must be rewritten when a note is written to a nested file or archived.
- An image pasted into a note that is then cancelled stays in `images/` unreferenced.
- Deciding whether an image is unused requires scanning all notes files and the archive.
