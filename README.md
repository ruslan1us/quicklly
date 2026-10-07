# Quicklly

A hotkey mini-notepad for Windows. Press a global hotkey anywhere, type a thought, hit
<kbd>Enter</kbd> — it is appended with a timestamp to today's markdown file, and you are
back in whatever you were doing.

![Quicklly input window](docs/screenshot.png)

- **Fast** — the input window pops up instantly on top of everything, already focused.
- **Plain text** — notes are ordinary `.md` files you can open in Obsidian or VS Code and sync
  with OneDrive or Git. No database.
- **Lightweight** — lives in the system tray.

## Installation

Download the installer (`Quicklly_x.y.z_x64-setup.exe`, or the `.msi`) from the
[latest release](https://github.com/ruslan1us/quicklly/releases/latest) and run it.
The installer is not code-signed yet, so Windows SmartScreen may warn you: click
**More info → Run anyway**.

## Usage

| Action                         | How                                         |
| ------------------------------ | ------------------------------------------- |
| Open the input window          | <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>N</kbd> (can be changed in the settings), or click the tray icon |
| Save the note                  | <kbd>Enter</kbd>                            |
| New line in the note           | <kbd>Shift</kbd>+<kbd>Enter</kbd>           |
| Previous / next saved note     | <kbd>↑</kbd> / <kbd>↓</kbd> in an empty field |
| Browse notes (see [Reader](#reader)) | <kbd>←</kbd> in an empty field        |
| Write a longer note in the Pad | <kbd>→</kbd> in an empty field, or `/pad`; <kbd>Ctrl</kbd>+<kbd>Enter</kbd> saves it as a note |
| Continue in the Pad            | <kbd>Ctrl</kbd>+<kbd>E</kbd> moves what you typed into the Pad; <kbd>←</kbd> in an empty Pad goes back |
| Search all notes               | Type `?` in an empty field, then a word; pick a result with <kbd>↑</kbd> <kbd>↓</kbd> and <kbd>Enter</kbd> to open it in the reader, <kbd>Backspace</kbd> on an empty search to leave |
| Cancel                         | <kbd>Esc</kbd>, or <kbd>Ctrl</kbd>+<kbd>C</kbd> when no text is selected |
| Dismiss, keeping the draft     | Click anywhere outside the window           |
| Open the notes folder          | Tray menu → **Open notes folder**           |
| Settings                       | Type `/config` and press <kbd>Enter</kbd>, or tray menu → **Settings…** |
| Help and version               | Type `/help` and press <kbd>Enter</kbd>    |
| What's new                     | Type `/changelog` and press <kbd>Enter</kbd> (see [CHANGELOG.md](CHANGELOG.md)) |
| Update                         | When a new version is out, the hint says so for two seconds; type `/update` and press <kbd>Enter</kbd> to install it now. Otherwise it is downloaded in the background and installed on the next start (unless **Auto update** is off in the settings) |
| Exit                           | Type `/exit` and press <kbd>Enter</kbd>, or tray menu → **Quit** |

## Where notes are stored

Each day gets its own file in `%USERPROFILE%\Documents\Quicklly\`:

```
Quicklly/
  2026-09-26.md
  2026-09-27.md
```

A file looks like this:

```markdown
# 2026-09-26

- 14:32 check why swap keeps growing on prod3
- 15:10 buy a birthday present
- 18:45 coffee tracker in the tray #idea
```

Files are UTF-8; new notes are appended to the end, and the heading is written with the first
note of the day. A multi-line note stays one list item, with the extra lines indented under it:

```markdown
- 19:20 groceries:
  milk
  bread
```

Alternatively, choose **Single inbox** in the settings to keep all notes in one `inbox.md`,
grouped by day:

```markdown
# Inbox

## 2026-09-26

- 14:32 check why swap keeps growing on prod3
- 15:10 buy a birthday present

## 2026-09-27

- 09:05 book a dentist appointment
```

A note with a `#tag` goes to the file of that tag instead: `buy milk #todo` is appended to
`todo.md`, grouped by day like the inbox. The tag stays in the note; with several tags, the
first one decides. A tag starts with a letter, so `#1` or `C#` are not tags.

The notes folder can be changed in the settings as well.

## Reader

Press <kbd>←</kbd> in an empty input window to browse your notes. The first screen lists the
notes files, either the one changed last first or by name (the
inbox, tag files and any other Markdown files in the folder, then the days, newest first), each with a progress bar of done notes; a file where every note is
done is crossed off. Open one to go through its notes one by one (a multi-line note is one
entry):

| Key                                  | Files                  | Notes                              |
| ------------------------------------ | ---------------------- | ---------------------------------- |
| <kbd>↑</kbd> <kbd>↓</kbd>            | Select a file          | Select a note                      |
| <kbd>Enter</kbd>                     | Open the file          | Edit the note                      |
| <kbd>Tab</kbd>                       | Sort: recent first / by name |                              |
| <kbd>Shift</kbd>+<kbd>Enter</kbd>    |                        | Edit the note in the Pad           |
| <kbd>Space</kbd>                     |                        | Mark as done / not done            |
| <kbd>Del</kbd>                       | Move to the Recycle Bin (press <kbd>Del</kbd> twice) | Delete (press <kbd>Del</kbd> twice) |
| <kbd>Ctrl</kbd>+<kbd>P</kbd>         |                        | Pin: keep the reader open over other windows, resizable and up to date |
| <kbd>←</kbd>                         |                        | Back to the files                  |
| <kbd>→</kbd>                         | Back to the input      |                                    |
| <kbd>Esc</kbd>, <kbd>Ctrl</kbd>+<kbd>C</kbd> | Close          | Close                              |

A done note is stored as a Markdown task, which Obsidian shows as a checked box:

```markdown
- [x] 15:10 buy a birthday present
```

Changes only touch that note; the rest of the file stays as it is. If the file was changed
elsewhere in the meantime, nothing is written and the reader reloads it.

## Pad

For longer notes there is the Pad: a bigger editor with line numbers. Open it with <kbd>→</kbd>
in an empty input window or `/pad`, or press <kbd>Ctrl</kbd>+<kbd>E</kbd> to carry on there with
what you already typed. It pops up in the top right corner (see **Pad position** in the settings),
or opens straight away with the hotkey if **Hotkey opens** is set to **Pad**.

| Key                                  | Action                                                   |
| ------------------------------------ | -------------------------------------------------------- |
| <kbd>Ctrl</kbd>+<kbd>Enter</kbd>     | Save the text as a note and start over                   |
| <kbd>Enter</kbd>                     | New line; in a list (`- `, `- [ ] `, `1. `) the next item |
| <kbd>Tab</kbd> / <kbd>Shift</kbd>+<kbd>Tab</kbd> | Indent / outdent                             |
| <kbd>Ctrl</kbd>+<kbd>P</kbd>         | Pin: keep the Pad open over other windows, movable and resizable |
| <kbd>←</kbd> in an empty Pad         | Back to the input window                                 |
| <kbd>Esc</kbd>, <kbd>Ctrl</kbd>+<kbd>C</kbd> | Close; the draft is kept, even across restarts   |

A note saved from the Pad keeps its blank lines and indentation. `#tags` and checkboxes are
highlighted as you type. In the [reader](#reader), <kbd>Shift</kbd>+<kbd>Enter</kbd> opens a note in
the Pad to edit it there.

## Settings

Open the settings with `/config` in the input window or from the tray menu. They are fully
keyboard-driven:

| Key                                  | Action                                     |
| ------------------------------------ | ------------------------------------------ |
| <kbd>↑</kbd> <kbd>↓</kbd>            | Select a setting                           |
| <kbd>←</kbd> <kbd>→</kbd>            | Switch between values                      |
| <kbd>Enter</kbd>                     | Edit: record a new hotkey, pick a folder   |
| <kbd>Del</kbd>                       | Reset the setting to its default           |
| <kbd>Esc</kbd>, <kbd>Ctrl</kbd>+<kbd>C</kbd> | Close                              |

| Setting            | Default                                     | Options                                      |
| ------------------ | ------------------------------------------- | -------------------------------------------- |
| Hotkey             | <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>N</kbd> | Any key or combination except <kbd>Esc</kbd> |
| Hotkey opens       | Quick line                                  | Quick line, Pad                              |
| Pad position       | Top right                                   | Top right, Top left, Bottom right, Bottom left, Center |
| Notes folder       | `%USERPROFILE%\Documents\Quicklly`          | Any folder                                   |
| Notes file         | One file per day                            | One file per day, or a single `inbox.md`     |
| Theme              | Default                                     | Default (dark), Default+ (dark purple), Light |
| Transparency       | 30%                                         | Off to 80%: how much of the blurred desktop shows through the windows |
| Scale              | 100%                                        | 80% to 150%                                  |
| Start with Windows | Off                                         | On, Off                                      |
| Auto update        | On                                          | On, Off (`/update` works either way)         |

## Building from source

Prerequisites (on Windows):

- [Node.js](https://nodejs.org/) LTS
- [Rust](https://rustup.rs/) (stable, MSVC toolchain)
- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
  with the Windows SDK
- WebView2 (preinstalled on Windows 10/11)

See the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for details.

```sh
npm install
npm run tauri dev     # run in development mode
npm run tauri build   # build the installer into src-tauri/target/release/bundle/
```

## License

[MIT](LICENSE)
