# Overview

Quicklly is a hotkey notepad for Windows, macOS and Linux. A global hotkey brings up a one-line
input window over every other window; the typed thought is appended with the time to a Markdown
file and the window disappears, so the user stays in the task at hand. Around that core the
application offers a larger editor (the Pad), a reader for browsing, completing, editing and
deleting notes, search, tags, due dates with reminders, an archive of completed notes and a
journal of recent work. All data lives in plain Markdown files in a folder the user chooses. This
page states the purpose, the principles that shape every change, the scope and the platforms, and
the direction of the project.

## Purpose

The design target is the time from a thought to a saved note: about two seconds, with no mouse
and no context switch. Everything else (reading, organising, reminding) exists to make the notes
captured that way useful later, without turning the application into a general note-taking
system.

## Product principles

| Principle | What it means in practice |
| --- | --- |
| Keyboard first | Every window is fully usable from the keyboard. Windows open focused where the user works (on the monitor with the active window) and close with <kbd>Esc</kbd> or <kbd>Ctrl</kbd>+<kbd>C</kbd>. The mouse works but is never required. |
| Plain Markdown files the user owns | Notes are ordinary UTF-8 `.md` files in a user-chosen folder. There is no database, no cache of note content and no hidden metadata. The files can be edited by hand, synced with OneDrive or Git, and read by other tools; the application tolerates hand edits. See [Data format](data-format.md) and [ADR 0002](decisions/0002-plain-markdown-files-as-the-only-storage.md). |
| Obsidian-compatible formats | Formats are chosen so that Obsidian renders the files correctly: done notes are Markdown tasks, due and done dates use the Obsidian Tasks emoji format, images are standard Markdown image links relative to the note file. See [ADR 0004](decisions/0004-obsidian-tasks-formats-for-due-and-done-dates.md). |
| Simplicity over guardrails | The simplest implementation wins. There are no input restrictions and no heuristics that guess the user's intent or the state of the outside world; the user decides. For example, `#tag` routing follows a fixed grammar rather than guessing, and a hotkey that cannot be registered is reported rather than replaced. |
| Minimal settings | A setting is added only where users genuinely differ (folder, file layout, date format, appearance). Behaviour that can have one good default has no setting. |
| Lightweight | The application lives in the tray, creates windows only when they are first needed, and keeps its installer small. See [ADR 0001](decisions/0001-tauri-2-over-electron.md) and [ADR 0008](decisions/0008-windows-created-lazily-from-async-commands.md). |
| Safe changes to user data | Destructive operations go to the Recycle Bin (the Trash on macOS and Linux), edits only touch the note the user saw and are refused if the file changed, and changes made in the reader can be undone. See [ADR 0010](decisions/0010-reader-changes-guarded-by-the-note-as-last-seen.md) and [ADR 0015](decisions/0015-deletions-go-to-the-recycle-bin.md). |

## Scope

In scope:

- Capturing notes: the input window, the Pad, pasted images, tags, dates, saving from scripts and
  `quicklly://` links.
- Working with notes: the reader (complete, edit, delete, undo, copy links, open images), search,
  reminders for due notes, the archive of completed notes and the journal of the last seven days.
- Running unobtrusively: tray icon (the menu bar on macOS), global hotkey, start at login, signed
  automatic updates.

The application runs on Windows, macOS (13.1 or later, one universal build) and Linux (x86-64,
AppImage, deb and rpm), from one code base: platform code sits behind `cfg(target_os)` in Rust
and the build-time `__PLATFORM__` constant in the pages ([Frontend](frontend.md#build-set-up)).
Where a platform can't do what Windows does, the application does the simplest honest thing
instead:

| Platform | Differences from Windows |
| --- | --- |
| macOS | ⌘ for the app's own shortcuts (⌃C still closes), ⌫ deletes in the reader; a menu-bar app without a Dock icon that hides itself when its last window goes, so the previous app gets the keyboard back ([Architecture](architecture.md#focus-on-macos)); HUD vibrancy instead of Acrylic; no window over a full-screen app; links saved by the running app ([ADR 0019](decisions/0019-links-saved-by-the-running-app-on-macos.md)); undo in the reader can't take images back out of the Trash; ad-hoc signed, not notarized |
| Linux | On Wayland the hotkey is a desktop shortcut that runs `quicklly` ([ADR 0020](decisions/0020-global-hotkey-on-wayland-via-a-desktop-shortcut.md)), and the compositor places the windows; no blur behind the windows; the tray is an AppIndicator menu (GNOME needs an extension); deb and rpm updates only on `/update` ([ADR 0021](decisions/0021-deb-and-rpm-updates-only-on-request.md)) |

The user-facing side of these is in the README's [Installation](../README.md#installation).

## Non-goals

- A database, an index or any storage other than the Markdown files.
- Account, cloud service or built-in sync; syncing is left to the user's tools.
- Rich text editing or Markdown rendering beyond highlighting links, tags, dates and images.
- Organisation through commands such as `/todo` or `/link`; organisation goes through `#tags`, and
  `/` commands control the application only.
- Capturing screenshots; images are pasted from the clipboard.
- Animations and automatic "what's new" pop-ups.
- Capturing text selected in the active window. This was prototyped and dropped: it required
  simulating <kbd>Ctrl</kbd>+<kbd>C</kbd> and restoring the clipboard.

## Direction

The following describes the intended direction at a high level. It is not a commitment to scope
or dates.

- **Version 1.0** is planned to include reminders at a time of day (a system notification for a
  note such as "tomorrow at 10 call the bank", extending the day-based dates and reminders),
  publication in winget, screenshots in the README, and ports to macOS and Linux.
- **After the macOS and Linux ports.** Known next steps, each only if hand testing shows the
  need: a non-activating panel (`NSPanel`) on macOS so the windows can show over full-screen apps;
  the `GlobalShortcuts` portal on Wayland, whose activation token would also help a desktop that
  refuses to focus the window; reading pasted images from the native clipboard where WebKitGTK
  doesn't pass them to the page.
- **Backlog ideas**, not scheduled: capturing context with a note (active window, browser URL),
  writing into an Obsidian vault's daily note, voice input, LLM-assisted triage of the inbox, Git
  sync.

## Note lifecycle

```mermaid
flowchart LR
    A["Input window / Pad / CLI"] -->|"save_note: - HH:MM text"| B{"First #tag?"}
    B -->|"none, daily mode"| D["YYYY-MM-DD.md"]
    B -->|"none, inbox mode"| I["inbox.md"]
    B -->|"#tag or #a/b"| T["tag.md or a/b.md"]
    D & I & T -->|"📅 date reached"| R["Reminders"]
    D & I & T -->|"Space in reader or reminders"| X["- [x] … ✅ YYYY-MM-DD"]
    X -->|"N days later"| AR["archive/same path.md"]
    X -->|"done in last 7 days"| J["Done journal"]
    AR --> J
```

Details of each step are in [Data format](data-format.md) and the [window pages](windows/README.md).
