# 0022. macOS and Linux alongside Windows

- **Status:** Accepted; amends [0001](0001-tauri-2-over-electron.md),
  [0012](0012-settings-in-tauri-plugin-store.md) and
  [0015](0015-deletions-go-to-the-recycle-bin.md)
- **Date:** 2026-10-19

## Context

Quicklly was written for Windows, and several records describe only Windows: ADR 0001 names
WebView2 as the web view, ADR 0012 a registry entry for starting with Windows, and ADR 0015 the
Windows Recycle Bin. The app is now also built for macOS and Linux from the same code.

## Decision

One code base for the three systems. What differs between them is behind
`cfg(target_os = …)` in Rust and the build-time platform constant in the pages
(`src/shared/platform.ts`), and the platform bundle settings are in `tauri.macos.conf.json` and
`tauri.linux.conf.json`. Each system keeps its own conventions where they differ, with no
emulation of the Windows behaviour:

- **Web view** (amends 0001): WKWebView on macOS and WebKitGTK on Linux, through Tauri as on
  Windows. The pages use only what all three support, or give WebKit its own rules (scrollbars,
  monospace fonts).
- **Start at login** (amends 0012): the autostart plugin writes a LaunchAgent on macOS and a
  `.desktop` file in `~/.config/autostart` on Linux, instead of the registry entry; the setting
  is the same.
- **Deletions** (amends 0015): files go to the Trash on macOS and Linux. On macOS they are moved
  through `NSFileManager` rather than Finder, which needs no Automation permission and plays no
  sound, but gives no "Put Back"; the Trash can't be listed there, so undo brings a note back but
  not its images.

## Consequences

- Platform code compiles only on its own system, so it is checked by CI on macOS and Linux
  runners, and the behaviour of windows, focus and the hotkey only by hand on those systems
  ([Testing](../testing.md)).
- Links on macOS and the hotkey on Wayland work differently and have their own records,
  [0019](0019-links-saved-by-the-running-app-on-macos.md) and
  [0020](0020-global-hotkey-on-wayland-via-a-desktop-shortcut.md); updates of deb and rpm
  packages, [0021](0021-deb-and-rpm-updates-only-on-request.md).

See [Overview](../overview.md) and [Release](../release.md).
