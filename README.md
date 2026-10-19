# Quicklly

A hotkey mini-notepad for Windows, macOS and Linux. Press a global hotkey anywhere, type a
thought, hit <kbd>Enter</kbd> — it is appended with a timestamp to today's markdown file, and you
are back in whatever you were doing.

![Quicklly input window](docs/screenshot.png)

- **Fast** — the input window pops up instantly on top of everything, already focused.
- **Plain text** — notes are ordinary `.md` files you can open in Obsidian or VS Code and sync
  with OneDrive or Git. No database.
- **Lightweight** — lives in the system tray (the menu bar on macOS).

## Installation

Everything is on the [latest release](https://github.com/ruslan1us/quicklly/releases/latest)
page. Installed copies update themselves (see **Update** under [Usage](#usage)).

### Windows

Download the installer (`Quicklly_x.y.z_x64-setup.exe`, or the `.msi`) and run it.
The installer is not code-signed yet, so Windows SmartScreen may warn you: click
**More info → Run anyway**.

### macOS

Download `Quicklly_x.y.z_universal.dmg` (one app for Apple silicon and Intel; macOS 13.1 or
later), open it and drag Quicklly to **Applications**. Start it from there, not from the disk
image, or it can't update itself.

The app is not notarized by Apple, so macOS blocks the first start ("Apple could not verify…"):
click **Done**, open **System Settings → Privacy & Security**, scroll down and click
**Open Anyway** next to Quicklly, then confirm. (Right-click → **Open** no longer gets past this
on macOS 15 and later.) Or remove the quarantine flag in Terminal:

```sh
xattr -dr com.apple.quarantine /Applications/Quicklly.app
```

Quicklly lives in the menu bar: it has no Dock icon and no entry in ⌘-Tab. Opening it again from
Finder, Launchpad or Spotlight shows the input window. At the first start macOS asks whether
Quicklly may access your Documents folder, where the notes go: allow it. As the app isn't signed
with an Apple developer certificate, macOS may ask again after an update.

On macOS:

- The app's own shortcuts take <kbd>⌘</kbd> instead of <kbd>Ctrl</kbd>: <kbd>⌘</kbd>+<kbd>E</kbd>,
  <kbd>⌘</kbd>+<kbd>V</kbd>, <kbd>⌘</kbd>+<kbd>Enter</kbd> in the Pad, <kbd>⌘</kbd>+<kbd>P</kbd>,
  <kbd>⌘</kbd>+<kbd>Z</kbd> in the reader. <kbd>⌃</kbd>+<kbd>C</kbd> (Control) still closes a
  window, since <kbd>⌘</kbd>+<kbd>C</kbd> copies, and <kbd>⌫</kbd> deletes in the reader, as Mac
  keyboards have no <kbd>Del</kbd>. The default hotkey is <kbd>⌃</kbd>+<kbd>⌥</kbd>+<kbd>N</kbd>.
- The windows can't show over an app in full screen: the hotkey switches to the desktop first.
- <kbd>⌘</kbd>+<kbd>Z</kbd> in the reader brings a deleted note back, but not its images: they
  stay in the Trash, from where you can drag them back into the `images` folder.
- To save notes from Terminal (see [From scripts and links](#from-scripts-and-links)), put the
  program on your `PATH` (create `/usr/local/bin` first if it doesn't exist):

  ```sh
  sudo ln -s /Applications/Quicklly.app/Contents/MacOS/quicklly /usr/local/bin/quicklly
  ```

### Linux

Three formats, all for x86-64, built on Ubuntu 24.04, so they need glibc 2.39 or later
(Ubuntu 24.04, Debian 13, Fedora 40 and newer):

- **AppImage** (`Quicklly_x.y.z_amd64.AppImage`): make it executable
  (`chmod +x Quicklly_x.y.z_amd64.AppImage`) and run it. It updates itself in place, so keep it in
  a folder you can write to. It isn't on your `PATH`: use its full path, or link it into
  `~/.local/bin` as `quicklly`.
- **deb** (Debian, Ubuntu): `sudo apt install ./Quicklly_x.y.z_amd64.deb`.
- **rpm** (Fedora, openSUSE): `sudo dnf install ./Quicklly-x.y.z-1.x86_64.rpm`.

The deb and rpm install `quicklly` on your `PATH`. They don't update at start, since installing a
package asks for your password: a new version is announced in the input window, and `/update`
installs it and asks for the password then.

**Tray.** On GNOME the tray icon needs the AppIndicator extension. Ubuntu ships it turned on; on
Fedora and other GNOME desktops install **AppIndicator and KStatusNotifierItem Support** (on Fedora
`sudo dnf install gnome-shell-extension-appindicator`, then turn it on in **Extensions**). KDE,
Xfce, Cinnamon and MATE show the icon as they are. A click on the icon opens its menu, which has
**New note**.

**Hotkey.** In an X11 session the hotkey works as on Windows. On Wayland, applications can't take
a global hotkey, so you bind one in your desktop to the command `quicklly` (for the AppImage, the
full path to it), and the settings say **Set in system settings: run quicklly**:

- GNOME: **Settings → Keyboard → View and Customize Shortcuts → Custom Shortcuts → Add Shortcut**;
- KDE Plasma: **System Settings → Keyboard → Shortcuts → Add New → Command or Script**;
- Sway or Hyprland: a `bindsym` or `bind` line that runs `exec quicklly`.

The key then opens what **Hotkey opens** in the settings names. If Quicklly isn't running, the
first press starts it in the tray; press again for the window.

On Wayland the desktop decides where windows go, so the Pad and the reminders may not open in
their corners, and a pinned Pad or reader keeps its size but neither its place nor staying on
top. There is no blur behind the windows on Linux: the transparency setting shows the desktop as
it is.

**Pasting images** needs WebKitGTK 2.52 or later. The deb and rpm use your system's WebKitGTK
(`libwebkit2gtk-4.1`), the AppImage the one it was built with.

**A blank or black window**, mostly with NVIDIA graphics or on Wayland, is a known WebKitGTK
problem. Start Quicklly from a terminal with one of these set, trying them in turn:

```sh
WEBKIT_DISABLE_DMABUF_RENDERER=1 quicklly
__NV_DISABLE_EXPLICIT_SYNC=1 quicklly
WEBKIT_DISABLE_COMPOSITING_MODE=1 quicklly
```

If one helps, set it for every start, for example in a file in `~/.config/environment.d/` (read at
login by GNOME and KDE). If the AppImage misbehaves where the deb or rpm doesn't, use the package.

## Usage

Keys are written as on Windows and Linux; on macOS the app's own shortcuts take <kbd>⌘</kbd>
instead of <kbd>Ctrl</kbd> (see [macOS](#macos)).

| Action                         | How                                         |
| ------------------------------ | ------------------------------------------- |
| Open the input window          | <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>N</kbd> (can be changed in the settings), or click the tray icon (on Linux, **New note** in its menu) |
| Save the note                  | <kbd>Enter</kbd>                            |
| New line in the note           | <kbd>Shift</kbd>+<kbd>Enter</kbd>           |
| Paste an image                 | <kbd>Ctrl</kbd>+<kbd>V</kbd> with an image in the clipboard (also in the Pad); see [Reader](#reader) |
| Pick an existing tag           | Type `#` and the start of a tag; pick one with <kbd>↑</kbd> <kbd>↓</kbd> and <kbd>Enter</kbd> or <kbd>Tab</kbd>, <kbd>Esc</kbd> closes the list (also in the Pad) |
| Add a date                     | Type `/date` and press <kbd>Enter</kbd> to pick a day in the calendar, or `/date tomorrow` / `/date 15/10/2026`; see [Dates](#dates) (also in the Pad) |
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
| Save a note from a script or a link | `quicklly.exe add "buy milk #todo"` (`quicklly add …` on macOS and Linux) or `quicklly://add?text=buy%20milk%20%23todo`; see [From scripts and links](#from-scripts-and-links) |
| Notes due                      | Type `/reminders` and press <kbd>Enter</kbd>; see [Reminders](#reminders) |
| What you did this week         | Type `/done` and press <kbd>Enter</kbd>; see [Done](#done) |
| Update                         | When a new version is out, the hint says so for two seconds; type `/update` and press <kbd>Enter</kbd> to install it now. Otherwise it is downloaded in the background and installed on the next start (unless **Auto update** is off in the settings; the Linux deb and rpm install only on `/update`) |
| Exit                           | Type `/exit` and press <kbd>Enter</kbd>, or tray menu → **Quit** |

## Where notes are stored

Each day gets its own file in `%USERPROFILE%\Documents\Quicklly\` on Windows,
`~/Documents/Quicklly/` on macOS and Linux (`~/Quicklly/` on a Linux system without a Documents
folder):

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

Tags can be nested with `/`: `#work/quicklly` goes to `work/quicklly.md`, in a `work` folder
of the notes folder (created when needed), at any depth. Images in such a note are linked from
that folder (`../images/…`), so Obsidian shows them. The reader sorted by name shows the folders
as a tree. `archive` and `images` are the app's own folders, so `#archive/x` and `#images/x` are
just `#archive` and `#images`.

The notes folder can be changed in the settings as well. The settings themselves and the Pad's
unsaved draft are kept in the app's data folder (`%APPDATA%\com.quicklly.app` on Windows); see
[Application files](docs/data-format.md#application-files) for every system.

### Dates

Type `/date` anywhere in a note (in the input window or the Pad) and press <kbd>Enter</kbd>: a
calendar opens on today. <kbd>←</kbd> <kbd>→</kbd> move by a day, <kbd>↑</kbd> <kbd>↓</kbd> by a
week, <kbd>PgUp</kbd> <kbd>PgDn</kbd> by a month, <kbd>Enter</kbd> picks the day and
<kbd>Esc</kbd> closes the calendar. `/date tomorrow` or `/date 15/10/2026` (in your date format)
puts the date in straight away. Either way `/date` is replaced by the date, `📅 15/10/2026`, and
you go on typing; the next <kbd>Enter</kbd> saves the note. One <kbd>Backspace</kbd> right after
a date removes all of it. Files keep the date as
`📅 2026-10-15`, the due date of [Obsidian Tasks](https://publish.obsidian.md/tasks/); the input
window, the Pad and the reader show it in the **Date format** of the settings.

### Reminders

Notes that are not done and have a date of today or earlier pop up in a small window in the
bottom right corner when Quicklly starts (also at login) and when a new day begins while it
runs: overdue ones first, then today's. The window stays on top without taking the focus, so it
doesn't get in the way of your typing; click it to use the keys. <kbd>↑</kbd> <kbd>↓</kbd> select
a note, <kbd>Space</kbd> marks it done (and takes it off the list), <kbd>S</kbd> moves its date
to tomorrow and <kbd>W</kbd> a week from today (which takes it off the list too),
<kbd>Enter</kbd> opens it in the [reader](#reader) and <kbd>Esc</kbd> closes the window; it
closes by itself once everything is done. <kbd>Tab</kbd> switches to the notes ahead: those with
a later date, soonest first, under a line with each day's date; the same keys work there except
<kbd>S</kbd> and <kbd>W</kbd>, and <kbd>Tab</kbd> again goes back. `/reminders` in the input
window opens it at any time, on the notes ahead when nothing is due.

### Archive

Marking a note done adds the day it was done to the file, `✅ 2026-10-13`, the done date of
[Obsidian Tasks](https://publish.obsidian.md/tasks/); marking it not done again takes it off.
The reader doesn't show that date, but how long the note has left, like `archive in 3 days`.
After a while (7 days, see **Archive done notes** in the
settings) done notes move out of their file into the same file in an `archive` folder in the
notes folder: `todo.md` → `archive/todo.md`, `2026-10-08.md` → `archive/2026-10-08.md`, grouped
by day like the original. This happens when Quicklly starts, when a new day begins and when the
setting is changed. The reader, the search and the reminders don't show the archive. Notes marked
done before this version have no done date, so they stay where they are.

### Done

`/done` in the input window lists the notes marked done in the last 7 days (today and the six
days before), from every notes file and the archive, under a line with the date each was done,
the latest day first. <kbd>C</kbd> copies the list as Markdown for a report, a `## <date>`
heading for each day and the notes under it; <kbd>↑</kbd> <kbd>↓</kbd> <kbd>PgUp</kbd>
<kbd>PgDn</kbd> scroll a long list and <kbd>Esc</kbd> closes it. Notes
marked done before v0.8.1 have no done date, so they are not listed.

## Reader

Press <kbd>←</kbd> in an empty input window to browse your notes. The first screen lists the
notes files, either the one changed last first or by name (the
inbox, tag files and any other Markdown files in the folder, then the days, newest first, then
each folder with its files the same way), each with a progress bar of done notes; a file where every note is
done is crossed off. Open one to go through its notes one by one (a multi-line note is one
entry):

| Key                                  | Files                  | Notes                              |
| ------------------------------------ | ---------------------- | ---------------------------------- |
| <kbd>↑</kbd> <kbd>↓</kbd>            | Select a file          | Select a note                      |
| <kbd>Enter</kbd>                     | Open the file          | Edit the note                      |
| <kbd>Tab</kbd>                       | Sort: recent first / by name |                              |
| <kbd>Shift</kbd>+<kbd>Enter</kbd>    |                        | Edit the note in the Pad           |
| <kbd>Space</kbd>                     |                        | Mark as done / not done            |
| <kbd>C</kbd>                         |                        | Copy the note's links              |
| <kbd>O</kbd>                         |                        | Open the note's images             |
| <kbd>Del</kbd>                       | Move to the Recycle Bin, the Trash on macOS and Linux (press <kbd>Del</kbd> twice) | Delete (press <kbd>Del</kbd> twice) |
| <kbd>Ctrl</kbd>+<kbd>Z</kbd>         | Undo                   | Undo                               |
| <kbd>Ctrl</kbd>+<kbd>P</kbd>         |                        | Pin: keep the reader open over other windows, resizable and up to date |
| <kbd>←</kbd>                         |                        | Back to the files                  |
| <kbd>→</kbd>                         | Back to the input      |                                    |
| <kbd>Esc</kbd>, <kbd>Ctrl</kbd>+<kbd>C</kbd> | Close          | Close                              |

A done note is stored as a Markdown task, which Obsidian shows as a checked box, with the day it
was done (see [Archive](#archive)):

```markdown
- [x] 15:10 buy a birthday present ✅ 2026-09-27
```

Changes only touch that note; the rest of the file stays as it is. If the file was changed
elsewhere in the meantime, nothing is written and the reader reloads it.

<kbd>Ctrl</kbd>+<kbd>Z</kbd> undoes the deletes, done marks and edits made in the reader while it
is open, one per press, latest first: the note comes back exactly as it was, with its day heading
and its images (taken back out of the Recycle Bin or the Trash; not on macOS, see
[macOS](#macos)), and the reader opens its file. A change to a
file that has changed again since, say by a new note from the input window, is not undone.

Links (`https://…`, or just a domain like `google.com` or `github.com/ruslan1us`) are
underlined as you type, in the input window and in the Pad. The reader and the search results show them short, without the protocol
and with only the start of the path (`https://github.com/ruslan1us/quicklly/releases` →
`github.com/rusl...`); the file keeps the whole link, and <kbd>C</kbd> copies the selected
note's links, one per line.

To add a screenshot or any other image, copy it and paste it with <kbd>Ctrl</kbd>+<kbd>V</kbd> in
the input window or the Pad: it shows as `[Image #1]` (then `[Image #2]`, …) and is saved right
away in an `images` folder next to the notes. The note gets a Markdown image instead of the
label, `![Image #1](images/2026-10-07_14-32-05.png)`, so Obsidian shows the picture; the reader
shows the label again, and <kbd>O</kbd> opens the selected note's images in the default viewer.
Deleting a note or a notes file in the reader, or taking an image out of a note, moves its images
to the Recycle Bin (the Trash) too, unless another note still links to them. So does an image
pasted but never saved: once the note is cleared or cancelled in the input window, or the Pad
saves a note without it, unless a note or the Pad draft still has it.

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

## From scripts and links

Scripts, AutoHotkey, a Stream Deck or a browser bookmark can save a note too, exactly as if it was
typed in the input window and saved with <kbd>Enter</kbd>: `#tags` send it to their file as usual.
No window opens, whether Quicklly is running or not.

From the command line, with `quicklly.exe` from the install folder: `%LOCALAPPDATA%\Quicklly\`
for the usual installer, `C:\Program Files\Quicklly\` for the `.msi` (or right-click Quicklly in
the Start menu → **Open file location**, then the shortcut's **Open file location**):

```powershell
& "$env:LOCALAPPDATA\Quicklly\quicklly.exe" add "buy milk #todo" | Out-Host
```

```bat
start "" /wait "%LOCALAPPDATA%\Quicklly\quicklly.exe" add "buy milk #todo"
```

It prints `Saved to todo.md` (or `Error: …`) and exits with code 0 (or 1). The text may also come
as several arguments, which are joined with spaces. Quicklly is a windowed app, so PowerShell and
the `cmd` prompt don't wait for it on their own: `| Out-Host` and `start "" /wait` make them wait
and show the output before the next prompt (a `.bat` file waits anyway).

As a link, from a browser, the Run dialog (<kbd>Win</kbd>+<kbd>R</kbd>) or anything that opens
URLs, with the text URL-encoded (`%23` for `#`, `%20` or `+` for a space):

```
quicklly://add?text=buy%20milk%20%23todo
```

In AutoHotkey v2, for example, a hotkey that saves a fixed note:

```autohotkey
^!m::Run '"' EnvGet("LOCALAPPDATA") '\Quicklly\quicklly.exe" add "call mom #todo"'
```

On macOS and Linux the program is `quicklly`: on your `PATH` with the deb or rpm, through the link
from [macOS](#macos) on a Mac, or the AppImage's own path. The shell waits for it as for any
command, so no `Out-Host` is needed:

```sh
quicklly add "buy milk #todo"
```

Links work on Linux as on Windows (`xdg-open "quicklly://add?text=…"`). On macOS a link
(`open "quicklly://add?text=…"` in Terminal, a browser, Shortcuts) goes to the running Quicklly,
which saves the note; it starts Quicklly first if it isn't running, and gives no output.

## Settings

Open the settings with `/config` in the input window or from the tray menu. They are fully
keyboard-driven:

| Key                                  | Action                                     |
| ------------------------------------ | ------------------------------------------ |
| <kbd>↑</kbd> <kbd>↓</kbd>            | Select a setting                           |
| <kbd>←</kbd> <kbd>→</kbd>            | Switch between values                      |
| <kbd>Enter</kbd>                     | Edit: record a new hotkey, pick a folder   |
| <kbd>Del</kbd> (<kbd>⌫</kbd> on macOS) | Reset the setting to its default         |
| <kbd>Esc</kbd>, <kbd>Ctrl</kbd>+<kbd>C</kbd> | Close                              |

| Setting            | Default                                     | Options                                      |
| ------------------ | ------------------------------------------- | -------------------------------------------- |
| Hotkey             | <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>N</kbd> | Any key or combination except <kbd>Esc</kbd>; on Wayland set in the desktop instead (see [Linux](#linux)) |
| Hotkey opens       | Quick line                                  | Quick line, Pad                              |
| Pad position       | Top right                                   | Top right, Top left, Bottom right, Bottom left, Center |
| Notes folder       | `Quicklly` in Documents (see [Where notes are stored](#where-notes-are-stored)) | Any folder           |
| Notes file         | One file per day                            | One file per day, or a single `inbox.md`     |
| Date format        | DD/MM/YYYY                                  | DD/MM/YYYY, MM/DD/YYYY, YYYY-MM-DD, DD.MM.YYYY: how `/date` dates are typed and shown |
| Archive done notes | After 7 days                                | Never, after 1, 3, 7, 14 or 30 days: when done notes move to `archive/` (see [Archive](#archive)) |
| Theme              | Default                                     | Default (dark), Default+ (dark purple), Light |
| Transparency       | 30%                                         | Off to 80%: how much of the blurred desktop (not blurred on Linux) shows through the windows |
| Scale              | 100%                                        | 80% to 150%                                  |
| Start with Windows (Start at login on macOS and Linux) | Off     | On, Off                                      |
| Auto update        | On                                          | On, Off (`/update` works either way)         |

## Building from source

Prerequisites on every system: [Node.js](https://nodejs.org/) LTS and
[Rust](https://rustup.rs/) (stable). Then:

- **Windows:** the MSVC Rust toolchain,
  [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
  with the Windows SDK, and WebView2 (preinstalled on Windows 10/11).
- **macOS:** the Xcode Command Line Tools (`xcode-select --install`).
- **Linux** (Debian, Ubuntu):
  `sudo apt install build-essential libwebkit2gtk-4.1-dev libdbus-1-dev libayatana-appindicator3-dev librsvg2-dev`.

See the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for details and other Linux
distributions.

```sh
npm install
npm run tauri dev     # run in development mode
npm run tauri build   # build the installers into src-tauri/target/release/bundle/
npm test              # run the frontend tests
```

The Rust tests run with `cargo test` in `src-tauri/`.

## Documentation

How Quicklly is built, its data format, conventions and design decisions are documented in
[docs/](docs/README.md).

## License

[MIT](LICENSE)
