# 0019. Links saved by the running app on macOS

- **Status:** Accepted; amends [0007](0007-cli-handled-in-a-fresh-process-before-tauri-starts.md)
- **Date:** 2026-10-19

## Context

[ADR 0007](0007-cli-handled-in-a-fresh-process-before-tauri-starts.md) saves `quicklly add …`
and `quicklly://add?text=…` in a fresh process that checks its arguments before Tauri starts. That
relies on the system starting a new process with the link as its argument, which Windows and Linux
do.

macOS does not. A link opened in a browser, with `open` in Terminal, from Shortcuts, Raycast or
Alfred, is delivered as an Apple Event to the running application, which LaunchServices starts
first if needed. The link never appears in `argv`, so the fresh-process path never sees it, and
before this decision a link on macOS only brought Quicklly forward and saved nothing. The
`quicklly add …` command line is unaffected: running the executable from a terminal does start a
new process.

## Decision

On macOS the running application saves links itself. In set-up it listens for opened URLs with
`tauri-plugin-deep-link`'s `on_open_url` (the plugin turns `RunEvent::Opened` into that event), and
also takes the URLs from `get_current()`, for a link that started the app before the listener was
registered. Each URL goes through the same `cli::link_text` as the command-line path and is saved
with the same `save_note` as the input window, which emits `notes-changed`. No window is opened.

ADR 0007 still holds for the command line on every platform and for links on Windows and Linux.

## Consequences

- Links work on macOS, with the same parsing rules as elsewhere, and open windows update right
  away.
- Whoever opened the link gets no result: there is no exit code or output, and a failure is only
  logged. A blank link saves nothing and reports nothing.
- A link starts Quicklly if it was not running and leaves it running in the menu bar, unlike
  Windows, where the process that saves the note exits.
- The note is saved with the running app's settings, not ones read from `settings.json`, which is
  the same unless the file was changed by hand.

See [CLI and links](../cli-and-links.md).
