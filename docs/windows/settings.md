# Settings

The settings window is a keyboard-driven list: one row per setting, the arrow keys switch between
values, <kbd>Enter</kbd> edits, <kbd>Del</kbd> resets. Every change is applied and saved at once;
there is no "Apply" step. The Rust side is the single source of truth: every command returns the
complete settings view, which the page renders as is. The list of settings and their defaults is in
the root README, [Settings](../../README.md#settings); the stored keys are in
[Data format](../data-format.md#settingsjson-keys).

## Files

| File | Role |
| --- | --- |
| [`src/windows/settings/index.html`](../../src/windows/settings/index.html) | Header, rows, error line, footer; links `themes.css` and `terminal.css` |
| [`src/windows/settings/settings.ts`](../../src/windows/settings/settings.ts) | Row definitions, keys, hotkey recording |
| [`src-tauri/src/settings.rs`](../../src-tauri/src/settings.rs) | Store access, the `SettingsView`, all setting commands |

## Rows and commands

Each row defines a label, how its value is shown, and optionally `cycle` (←/→), `edit`
(<kbd>Enter</kbd>) and `reset` (<kbd>Del</kbd>).

| Row | ←/→ | Enter | Del | Side effect in Rust |
| --- | --- | --- | --- | --- |
| Hotkey | | Record a new hotkey (`set_hotkey`) | `reset_hotkey` | Unregister the old shortcut, register the new one; on failure re-register the old one and return an error |
| Hotkey opens | `set_hotkey_target` | Same as → | Quick line | |
| Pad position | `set_pad_position` | Same as → | Top right | |
| Notes folder | | Folder picker (`pick_notes_dir`) | `reset_notes_dir` | |
| Notes file | `set_mode` | Same as → | One file per day | |
| Date format | `set_date_format` | Same as → | DD/MM/YYYY | Emits `date-format-changed` |
| Archive done notes | `set_archive_after` | Same as → | After 7 days | Archives what is now due |
| Theme | `set_theme` | Same as → | Default | Emits `appearance-changed` |
| Transparency | `set_transparency` (0–80 % in steps) | Same as → | 30 % | Emits `appearance-changed` |
| Scale | `set_scale` (80, 90, 100, 110, 125, 150 %) | Same as → | 100 % | Re-zooms all windows, resizes an unpinned Pad, emits `appearance-changed` |
| Start with Windows | `set_autostart` | Same as → | Off | Adds or removes the `Run` registry entry through the autostart plugin |
| Auto update | `set_auto_update` | Same as → | On | On: check now; off: forget the announced release |

## Keys

| Key | Action |
| --- | --- |
| <kbd>↑</kbd> <kbd>↓</kbd> | Select a row (wraps) |
| <kbd>←</kbd> <kbd>→</kbd> | Previous or next value |
| <kbd>Enter</kbd>, <kbd>Space</kbd>, click on a value | Edit, or next value for rows without an editor |
| <kbd>Del</kbd>, <kbd>Backspace</kbd> | Reset to the default |
| <kbd>Esc</kbd>, <kbd>Ctrl</kbd>+<kbd>C</kbd> | Close |

## Behaviour

- **Hotkey recording.** While recording, held modifiers are shown (`Ctrl+Alt+…`); the first
  non-modifier key completes the combination, sent as modifiers plus `KeyboardEvent.code`, for
  example `Ctrl+Alt+KeyN`, and parsed by `hotkey::parse`. <kbd>Esc</kbd> without modifiers, a click
  or leaving the window cancels. Any key the global-shortcut plugin accepts is allowed, with or
  without modifiers; there is no restriction to "safe" combinations.
- **Folder picker.** `pick_notes_dir` is async and opens a blocking folder dialog parented to the
  settings window, starting in the current folder. Cancelling keeps the current folder. Changing
  the folder does not move existing notes.
- **Errors** (for example a hotkey already taken by another application) appear under the list;
  the window refits to make room.
- The window fits its content without a height limit and is shown with `reveal()`.
