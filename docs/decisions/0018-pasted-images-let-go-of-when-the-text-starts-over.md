# 0018. Pasted images that are never saved are let go of when the text starts over

- **Status:** Accepted; amends [0006](0006-pasted-images-saved-as-files-next-to-the-notes.md)
- **Date:** 2026-10-14

## Context

[ADR 0006](0006-pasted-images-saved-as-files-next-to-the-notes.md) saves a pasted image to
`images/` at the moment of pasting. An image whose note is then cancelled, or whose label is
deleted before saving, stayed there with nothing linking to it, and such files piled up.

Two ways out were considered:

- **Write the image only when the note is saved.** The bytes would wait in the window. This does
  not fit the Pad: its draft is written to disk 300 ms after typing and must survive a restart,
  so it needs a path to link to, and so does text moved from the input window with
  <kbd>Ctrl</kbd>+<kbd>E</kbd>. It would need a second place for unsaved images and a conversion
  for it at every boundary.
- **Sweep `images/` for files nothing links to**, at start or once a day. The folder may hold
  files the user put there, and it is often synced (OneDrive, Git): a notes file that has not
  arrived yet from another computer would make its images look unused. A text still in a window,
  or in its undo history, is invisible to a sweep. Guarding against that needs age thresholds,
  which guess.

## Decision

Images keep being saved on paste. Each window holds the images that went into its text as
`images/…`: the input window those pasted into the note; the Pad those pasted into it and those of
its draft whenever it loads the draft. When the window's text starts over, which also drops its
undo history, it lets go of the images it held (`releaseImages`, command
`trash_unsaved_images`):

- the input window when its note is cleared: saved, cancelled with <kbd>Esc</kbd> or
  <kbd>Ctrl</kbd>+<kbd>C</kbd>, or left for another window; images moved to the Pad with
  <kbd>Ctrl</kbd>+<kbd>E</kbd> are handed over to it instead;
- the Pad when it saves a note, and when it loads the draft or a note to edit in its place.

Of the images let go of, the Rust side moves to the Recycle Bin those that nothing mentions: no
notes file, archive and tag folders included, not the Pad draft on disk, not the input window's
history. "Mentions" means the image's file name appears in the text, ignoring case, however it is
linked. Only files right in the top-level `images/` folder are ever considered.

## Consequences

- Images pasted and never saved go to the Recycle Bin as soon as the text they were pasted into is
  gone, and only then; images the user can still get back in a window stay.
- No image the window did not hold is ever touched, so files added to `images/` by hand or by
  sync are safe.
- An image left in the input window when the application quits, or deleted from a pinned Pad
  draft that is never saved or reloaded, stays in `images/`.
- Deciding costs a scan of all notes files, as in the reader; it runs only when a window held
  images.
- A second rule for "unused" exists next to the reader's (exact links resolved per file); the
  name rule is the more cautious one because it decides about files the user never saw in a note.
