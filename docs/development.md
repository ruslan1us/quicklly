# Development

This page covers what is needed to build and change Quicklly: prerequisites, commands, the checks
every change must pass, the code and repository conventions, the known pitfalls that have caused
real defects, and step-by-step checklists for the most common extensions (a window, a command, a
setting). Development happens natively on Windows, not in WSL, because the tray, the global hotkey
and the windows must be native.

## Prerequisites

- Node.js LTS and npm.
- Rust stable with the MSVC toolchain, plus `rustfmt` and `clippy`.
- Microsoft C++ Build Tools with the Windows SDK.
- WebView2 (preinstalled on Windows 10 and 11).

See the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for details.

## Commands

| Command | Where | Purpose |
| --- | --- | --- |
| `npm install` | root | Install frontend and CLI dependencies |
| `npm run tauri dev` | root | Run the app with the Vite dev server (port 1420) |
| `npm run build` | root | Type-check (`tsc`) and build the pages into `dist/` |
| `npm test` | root | Frontend tests (Vitest) |
| `npm run tauri build` | root | Release build and installers; needs the update signing key (see [Release](release.md)) |
| `npm run tauri build -- --no-bundle` | root | Release executable without installers or signing |
| `cargo fmt --check` | `src-tauri/` | Formatting |
| `cargo clippy --all-targets -- -D warnings` | `src-tauri/` | Lints, warnings are errors |
| `cargo test` | `src-tauri/` | Rust unit tests |

A development build and an installed release share the identifier `com.quicklly.app`, so they use
the same data folder (`%APPDATA%\com.quicklly.app`), the same settings and the same notes folder.

## Checks before every commit

All of the following must pass; CI ([`ci.yml`](../.github/workflows/ci.yml)) runs the same on
Windows, macOS and Linux (`windows-latest`, `macos-latest`, `ubuntu-24.04`) for every push to a
branch and every pull request (clippy and test with `--locked`):

```sh
npm run build
npm test
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Code behind `cfg(target_os = "macos")` or `cfg(target_os = "linux")` is compiled only on those
runners, so push the branch and wait for CI before merging such a change.

## Conventions

- **Small commits.** One logical change per commit, so each can be reviewed and reverted alone.
  Versions group several features; commits stay small.
- **Conventional Commits**: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `style:`, `ci:`,
  `chore:` (for example `chore: release v0.8.1`).
- **English** for everything in the repository: code, comments, documentation, commit messages,
  issues and release notes.
- **Rust** passes `cargo fmt` and `cargo clippy` with no warnings.
- **Comments** explain behaviour and the reason for it in full sentences, at the density of the
  surrounding code: doc comments on functions, constants and types, short line comments for
  non-obvious steps. Existing comments use British spelling (`colour`, `centre`, `recognised`).
- **Simplicity.** No input restrictions and no heuristics that guess the user's intent or the
  state of the outside world. See [Overview](overview.md#product-principles).
- **User-visible changes** get an entry under `## [Unreleased]` in
  [`CHANGELOG.md`](../CHANGELOG.md), and the root [README](../README.md) and the in-app help
  ([`src/windows/help/index.html`](../src/windows/help/index.html)) are updated where they describe
  the behaviour.
- **Documentation** in `docs/` is updated in the same change as the code it describes.
- **Agents** do not commit, tag, push or publish without an explicit instruction from the
  maintainer, and do not run the application or touch the user's notes or settings; manual
  verification is done by the maintainer.

## Known pitfalls

Each of these has caused a defect before.

1. **Create windows only from async commands.** Creating a window from a synchronous
   `#[tauri::command]` deadlocks on Windows. Any command that can create a window is `async` and
   carries the comment "Async on purpose: creating a window from a sync command deadlocks on
   Windows."
2. **Create hidden windows with `focused(false)`.** Otherwise creating the window takes the
   keyboard focus from the application in use, notably at start. Closing a window on focus loss is
   only done for a visible window: Windows can report a focus loss while a window is still being
   set up hidden.
3. **Import `terminal.css` from the popup's own stylesheet.** A popup's CSS file starts with
   `@import "../../shared/terminal.css";`. Linking both stylesheets separately lets the release
   build order them differently, so the shared rules override the window's own. The development
   server does not show this; only a release build does.
4. **Scroll the body, not the window, in popups.** In a transparent window the window's own
   scrollbar gets a black track. `terminal.css` makes `body` the scroll container, and window
   height is measured on `main`, not on `body`.
5. **Use `overflow: clip` on the input window's body.** With `hidden` the page can still scroll to
   the caret when the textarea grows a moment before the window, which makes the text jump.
6. **Replaced icons are not picked up by an incremental build.** After changing files in
   `src-tauri/icons/`, run `cargo clean -p quicklly` before building.
7. **Edit code with file-editing tools or script files.** A shell heredoc can turn `\n` escape
   sequences in source code into real line breaks.
8. **Register every new window label in `capabilities/default.json`.** Without it the page's
   window API calls (show, hide, set size, close) are rejected.
9. **Change text in fields with `document.execCommand`**, not by assigning `value`, wherever the
   user may want to undo the change with <kbd>Ctrl</kbd>+<kbd>Z</kbd>. See
   [Frontend](frontend.md#editing-conventions).

## Checklist: add a window

Using the done journal as the model:

1. Create `src/windows/<name>/index.html`, `<name>.ts` and `<name>.css`. The HTML links
   `../../shared/themes.css` and `./<name>.css` and loads `./<name>.ts` as a module; the content is
   wrapped in `<main>` with a `<header data-tauri-drag-region>` and a footer for key hints.
2. Start `<name>.css` with `@import "../../shared/terminal.css";`.
3. In `<name>.ts`: `followTheme(() => void fitToContent(MAX_HEIGHT))`, then load data, render,
   `fitToContent`, then `reveal()`; close with `appWindow.close()` on <kbd>Esc</kbd> and on
   <kbd>Ctrl</kbd>+<kbd>C</kbd> (compare `e.code === "KeyC"`).
4. Add the page to `build.rolldownOptions.input` in [`vite.config.ts`](../vite.config.ts).
5. Add the label to `windows` in
   [`src-tauri/capabilities/default.json`](../src-tauri/capabilities/default.json).
6. In Rust, add a label constant and an `async` command that calls
   `show_popup(&app, LABEL, "<name>", "Quicklly <Title>")`, plus any data commands; register them
   in `generate_handler!` in [`lib.rs`](../src-tauri/src/lib.rs).
7. If the window is opened from the input window, add the `/command` branch in `save()` in
   [`main.ts`](../src/windows/input/main.ts) using `switchTo`.
8. If the window shows notes, subscribe to `notes-changed` where it should stay current, and add its
   label to `notes_changed` in `lib.rs`.
9. Add a test file `tests/<name>.test.ts` (see [Testing](testing.md)), a window page under
   [`docs/windows/`](windows/README.md), and update the help, README and changelog.

## Checklist: add a command

1. Write the logic as a function over paths, strings and dates in the module it belongs to, with
   unit tests.
2. Add a `#[tauri::command]` wrapper. Resolve the notes folder with `settings::notes_dir`; validate
   any file name from the page with `reader::notes_file` (or work through functions that do); map
   errors to `String` with context.
3. Make it `async` if it may create a window. Make it `async` too if it only reads and may read
   many files (a scan of the notes folder), with its work in `in_background(move || …).await`, so
   it runs off the main thread. Keep a command that writes notes files sync, so writes stay
   one at a time on the main thread (see
   [Architecture](architecture.md#threads-of-the-commands)).
4. If it changes a notes file, call `notes_changed(&app)` after writing; if it changes an existing
   note, go through `reader::edit_note` so it is guarded and returns a `Change`.
5. Register it in `generate_handler!`.
6. Call it from the page with `invoke("name", { camelCaseArgs })`, show errors in the window, and
   mock it in the page's tests.
7. If the page needs a new core window permission, add it to
   [`capabilities/default.json`](../src-tauri/capabilities/default.json); application commands need
   none.

## Checklist: add a setting

1. In [`settings.rs`](../src-tauri/src/settings.rs): a key constant; a type with `serde` names that
   match the strings the page uses and a `Default` that is the default value; a getter with
   `stored(app, KEY)`.
2. Add the field to `SettingsView` and `view()`.
3. Add a `set_<name>` command that stores the value with `store_setting`, applies side effects
   (emit an event if open windows must react), and returns `view(&app)`; register it.
4. Add a test that pins the serialised strings, like
   `date_formats_are_stored_as_the_frontend_writes_them`.
5. In [`settings.ts`](../src/windows/settings/settings.ts): add the field to `Settings` and a `Row`
   with `value`, `cycle` or `edit`, and `reset` to the default.
6. If saving notes depends on it, read it in `notes_settings_in` too, so the CLI behaves the same.
7. Document the key in [Data format](data-format.md#settingsjson-keys), the setting in the README's
   settings table, and add a changelog entry.
