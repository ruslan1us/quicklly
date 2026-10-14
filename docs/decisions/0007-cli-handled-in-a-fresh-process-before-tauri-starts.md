# 0007. CLI and links handled in a fresh process before Tauri starts

- **Status:** Accepted
- **Date:** 2026-10-13

## Context

Scripts, AutoHotkey, Stream Deck buttons and browser bookmarks should be able to save a note
(`quicklly.exe add …`, `quicklly://add?text=…`). Windows starts a new process of the executable for
both. The usual Tauri approach forwards arguments to the running instance through the
single-instance plugin, which only works while the application runs and gives the caller no result.

## Decision

Check the arguments at the top of `run()`, before the Tauri builder is created. A note request is
saved by that process with the same `notes::append_note` as the input window, using the notes
folder and mode read directly from `settings.json`; the process prints the result, returns an exit
code and exits. It never contacts a running instance.

## Consequences

- Saving works whether or not the application is running, and scripts get `Saved to <file>` or
  `Error: …` with exit code 0 or 1.
- The CLI path must not depend on Tauri state or plugins; settings that affect saving must also be
  readable through `settings::notes_settings_in`.
- A running instance is not notified of the new note; open views see it on their next reload.
- The release executable is a GUI-subsystem program, so the CLI attaches to the parent console for
  output, and shells must be told to wait (documented in the README).

See [CLI and links](../cli-and-links.md).
