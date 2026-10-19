# 0021. deb and rpm updates installed only on request

- **Status:** Accepted; amends [0013](0013-signed-auto-updates-via-github-releases.md)
- **Date:** 2026-10-19

## Context

[ADR 0013](0013-signed-auto-updates-via-github-releases.md) downloads a newer release in the
background and installs it at the next start. On Windows that runs the per-user NSIS installer
without questions, and on macOS and for a Linux AppImage the updater plugin replaces the app in
place, so an update at start goes unnoticed.

Linux is released in three formats: an AppImage, a deb and an rpm. The plugin installs a deb or
rpm with `pkexec dpkg -i` or `pkexec rpm -U` (falling back to a graphical or terminal `sudo`
prompt), because packages are installed system-wide. That asks for the root password. Quicklly
usually starts at login, so an install at start would show a password prompt with no context
before the user has done anything, and could be mistaken for something malicious.

## Decision

Which kind of install is running is a fact: the bundler stamps the bundle type into the binary,
and `tauri::utils::platform::bundle_type` reads it. An AppImage installs a downloaded release at
start like the other platforms. A deb or rpm install downloads and announces it the same way, but
installs it only on `/update`, when the user has asked and expects the password prompt. Every
other platform and format keeps the behaviour of ADR 0013.

## Consequences

- No root password prompt appears at login; a deb or rpm user who never runs `/update` stays on
  the installed version, with the newer release announced in the input window.
- The deb and rpm still update from their own package (`linux-x86_64-deb`, `linux-x86_64-rpm` in
  `latest.json`), so the system package manager keeps track of the installed version.
- The rule is one check of the bundle type in
  [`updater.rs`](../../src-tauri/src/updater.rs) (`installs_at_startup`); a deb or rpm installed
  from a distribution's own repository would be updated by that repository instead.

See [Architecture](../architecture.md#updates) and [Release](../release.md#signed-updates).
