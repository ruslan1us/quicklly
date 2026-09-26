# Quicklly

A hotkey mini-notepad for Windows. Press a global hotkey anywhere, type a thought, hit
<kbd>Enter</kbd> — it is appended with a timestamp to today's markdown file, and you are
back in whatever you were doing.

![Quicklly input window](docs/screenshot.png)

- **Fast** — the input window pops up instantly on top of everything, already focused.
- **Plain text** — notes are ordinary `.md` files you can open in Obsidian or VS Code and sync
  with OneDrive or Git. No database.
- **Lightweight** — lives in the system tray.

## Usage

| Action                         | How                                         |
| ------------------------------ | ------------------------------------------- |
| Open the input window          | <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>N</kbd> (can be changed in the settings), or click the tray icon |
| Save the note                  | <kbd>Enter</kbd>                            |
| Cancel                         | <kbd>Esc</kbd>                              |
| Dismiss, keeping the draft     | Click anywhere outside the window           |
| Open the notes folder          | Tray menu → **Open notes folder**           |
| Settings                       | Tray menu → **Settings…**                   |
| Exit                           | Tray menu → **Quit**                        |

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
note of the day.

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

The notes folder can be changed in the settings as well (tray menu → **Settings…**).

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
