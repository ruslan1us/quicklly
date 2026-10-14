# 0012. Settings in tauri-plugin-store

- **Status:** Accepted
- **Date:** 2026-09-26

## Context

The application needs a few persistent settings (notes folder, file mode, hotkey, appearance) and
some window state (pin state, bounds, sort order). The notes folder must not hold application
state ([ADR 0002](0002-plain-markdown-files-as-the-only-storage.md)).

## Decision

Keep settings and window state in `settings.json` in the application data folder through
`tauri-plugin-store`. The Rust side owns all access: typed getters return the default for a missing
or invalid value, and setter commands save immediately and return the full settings view. Start
with Windows is the exception; it is the registry entry managed by `tauri-plugin-autostart`.

## Consequences

- Settings are a small JSON file the user can inspect; defaults need not be written.
- Enum values are stored as the strings the frontend uses, pinned by tests.
- Processes without the plugin (the CLI) read the file directly with `serde_json`
  ([ADR 0007](0007-cli-handled-in-a-fresh-process-before-tauri-starts.md)).
- Getters read the store on every call; hot paths such as window events keep pin state in managed
  state instead.
