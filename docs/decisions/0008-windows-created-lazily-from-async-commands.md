# 0008. Windows created lazily from async commands; input and Pad hidden, not destroyed

- **Status:** Accepted
- **Date:** 2026-09-26

## Context

The input window must appear instantly on the hotkey. Other windows (settings, reader, help, …)
are used occasionally, and each WebView2 window costs memory while it exists. Creating a window
inside a synchronous Tauri command deadlocks on Windows. A window created hidden but focused takes
the keyboard from the application in use.

## Decision

- The input window is declared in `tauri.conf.json`, created hidden and unfocused at start, and
  only shown and hidden afterwards. The Pad, once created, is also only hidden.
- Every other window is created on first use (`show_popup`, `pad::show`, `reminders::show`) and
  closed, not hidden, when dismissed.
- Every command that may create a window is `async`, with a comment stating why.
- Windows are created with `visible(false)` (and `focused(false)` for popups) and revealed by
  their page once themed, filled and sized.

## Consequences

- No memory is spent on windows that are not open; the input window and the Pad keep their text
  and undo history between uses.
- Requests to a window that may not exist yet (open the reader on a note, edit a note in the Pad)
  go through managed state plus an event, so they work whether or not the window exists
  ([Architecture](../architecture.md#events)).
- Popups start with a short delay while the page loads, hidden, so they never flicker at the
  wrong size.
- New window commands must follow the same rules; see the pitfalls in
  [Development](../development.md#known-pitfalls).
