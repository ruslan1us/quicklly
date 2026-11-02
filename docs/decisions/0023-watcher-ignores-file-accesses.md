# 0023. The notes folder watcher ignores file accesses

- **Status:** Accepted; amends [0017](0017-watch-the-notes-folder-for-changes.md)
- **Date:** 2026-11-01

## Context

[ADR 0017](0017-watch-the-notes-folder-for-changes.md) watches the notes folder with
`notify-debouncer-mini`. Its `add_event` counts every event for its paths and drops the kind, which
did no harm on Windows and macOS, where `notify` reports only changes. On Linux `notify` watches
with inotify's `OPEN` mask too, so reading a notes file was reported as a change. `notes-changed`
made the windows read the files again, which was reported again: an endless reload in the reader,
the reminders and the done journal. In the reader it showed as the scroll jumping back to the
selected note about half a second after each move of the mouse wheel.

The debouncer has no option to filter events by kind, and it hands on only paths.

## Decision

The watcher uses `notify` directly, with a channel into a thread of its own that does the debouncing
([`watcher.rs`](../../src-tauri/src/watcher.rs)). An event of the kind `EventKind::Access` is not a
change, on any system. The windows are told 300 ms after the last event that counts; the thread
ends when the watcher is dropped, which closes the channel. What counts as a notes file, and the
rest of ADR 0017, are unchanged.

## Consequences

- Reading a notes file no longer reloads the windows. The app's own writes still give the second
  `notes-changed` that ADR 0017 describes.
- A write reports a `Modify` event as well as the close that is ignored. Only a change that reports
  no `Modify`, such as a file written through a memory mapping, would go unseen on Linux; nothing
  known saves notes that way.
- The 300 ms are now counted across all notes files rather than for each path, and events that don't
  count don't extend them. A file changing without a pause of 300 ms is reported only once it stops,
  where the debouncer also reported it while it went on.
- One dependency less (`notify-debouncer-mini`), for about twenty lines of our own.

See [Backend](../backend.md#watcher).
