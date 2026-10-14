# CLI and links

Notes can be saved from outside the application: `quicklly.exe add <text>` from scripts and
terminals, and `quicklly://add?text=<text>` links from browsers, the Run dialog, AutoHotkey or a
Stream Deck. Both save the note exactly as <kbd>Enter</kbd> in the input window would, through the
same `notes::append_note`, and neither opens a window. The note is saved by the process that the
invocation starts, before Tauri is initialised, whether or not the application is already running.
The user-facing description is in the root README,
[From scripts and links](../README.md#from-scripts-and-links).

Implementation: [`src-tauri/src/cli.rs`](../src-tauri/src/cli.rs), called at the top of `run()` in
[`lib.rs`](../src-tauri/src/lib.rs). See also
[ADR 0007](decisions/0007-cli-handled-in-a-fresh-process-before-tauri-starts.md).

## Requests

`cli::note_request(args)` decides whether the command line is a note request. `args[0]` is the
executable.

| Arguments | Result |
| --- | --- |
| `add` followed by words | A request; the words are joined with single spaces |
| `add` alone, or only blank words | A request for a blank note, which fails with an error |
| One argument that is a `quicklly://add` link | A request with the link's `text` parameter |
| A `quicklly://add` link without `text`, or with an empty one | A request for a blank note |
| Nothing, or anything else (`--flag`, `remove x`, other links, a link plus more arguments) | Not a request: the application starts normally |

A blank request is still a request so that a script waiting for the process gets an error instead
of starting the application.

### Link rules

- Scheme `quicklly` and host `add`, compared case-insensitively; the path must be empty or `/`.
- Other query parameters are ignored; the first `text` parameter is used.
- The text is URL-decoded as a form value: `%20` and `+` are spaces, `%23` is `#`, `%2B` is `+`,
  UTF-8 sequences such as Cyrillic text are decoded.
- `quicklly:add?…` (without `//`), `quicklly://add/more`, `quicklly://open` and other schemes are
  not requests.

## Saving

`cli::save(identifier, text)`:

1. Attaches to the parent console if the process has no standard output or error handle (see
   below).
2. Reads the notes folder and the mode directly from `%APPDATA%\<identifier>\settings.json`
   (`settings::notes_settings_in`), because the store plugin is not running. A missing or unreadable
   file means the defaults: `Documents\Quicklly` and daily files.
3. Calls `notes::append_note` with the local time, so tags, nested folders, headings and
   normalisation are identical to the input window.
4. Prints `Saved to <file>` (the notes file relative to the notes folder, with `/`) on standard
   output and returns exit code `0`, or prints `Error: <message>` on standard error and returns `1`
   (including `Error: The note is empty`).

Output is written with `writeln!` and its result ignored, because `println!` panics when the
reader of the pipe has gone away.

## Console output from a windowed executable

The release executable uses the Windows GUI subsystem, so it has no console of its own and receives
output handles only when its output is redirected. When a handle is missing, `attach_console`
attaches to the parent process's console (`AttachConsole(ATTACH_PARENT_PROCESS)`) so that the
output appears in the terminal that started it.

Shells do not wait for a GUI-subsystem program: PowerShell and `cmd` print their next prompt
immediately and the output appears after it. Piping (`| Out-Host` in PowerShell) or
`start "" /wait` in `cmd` makes the shell wait; batch files wait anyway. This is documented for
users rather than worked around: the GUI subsystem is kept because a console-subsystem executable
opens a console window whenever it is started normally.

## Registration of the scheme

The `quicklly` scheme is declared under `plugins.deep-link.desktop.schemes` in
[`tauri.conf.json`](../src-tauri/tauri.conf.json). The NSIS and MSI installers register it for the
installed executable. A debug build registers it for itself at start (`register_all` under
`#[cfg(debug_assertions)]`), which points links at the development binary until the next install
or debug run. Windows passes the link as the executable's single argument, which `note_request`
handles before anything else starts.

## Interaction with a running instance

The request is never forwarded to a running instance. The new process writes the file and exits
before the single-instance plugin is registered. Consequences:

- Saving works the same whether the application is running, not running, or not yet started after
  sign-in.
- A running instance is not notified: an open reader or reminders window shows the note after its
  next reload, and the note is not added to the input window's history.
- Two processes can append to the same file at the same moment. Appends are small single writes,
  but there is no file lock.
