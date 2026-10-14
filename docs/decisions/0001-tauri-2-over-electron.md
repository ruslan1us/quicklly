# 0001. Tauri 2 over Electron

- **Status:** Accepted
- **Date:** 2026-09-26

## Context

Quicklly runs permanently in the tray and must show its input window instantly on a global
hotkey. It needs native features (tray icon, global shortcut, single instance, start with Windows,
signed self-update, acrylic window effects, Win32 calls for the active window) and a UI that is
quick to build and style. A resident utility should have a small installer and a small memory
footprint. Electron bundles Chromium and Node.js with every installation; Tauri uses the system
WebView2 and a Rust backend.

## Decision

Build the application with Tauri 2: a Rust backend with the official plugins (global-shortcut,
single-instance, store, dialog, autostart, updater, opener, deep-link) and a Vite + TypeScript
frontend rendered in WebView2. Use no frontend framework while the UI stays small.

## Consequences

- Small installers and a low resident footprint; WebView2 is preinstalled on Windows 10 and 11.
- File handling, parsing and system integration are written in Rust and unit-tested there; the
  frontend talks to it through typed commands and events (see [Architecture](../architecture.md)).
- Tauri-specific constraints apply, notably that windows must be created from async commands on
  Windows ([ADR 0008](0008-windows-created-lazily-from-async-commands.md)).
- Ports to macOS and Linux are possible with the same code base but need platform work (window
  effects, hotkeys on Wayland, Recycle Bin restore).
