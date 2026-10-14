# 0013. Signed automatic updates via GitHub Releases

- **Status:** Accepted
- **Date:** 2026-09-27

## Context

The application is distributed as installers on GitHub Releases and runs unattended in the tray,
so users would otherwise stay on old versions. An update mechanism must not let anyone but the
maintainer push code to users, and must not interrupt work.

## Decision

Use `tauri-plugin-updater` with the GitHub `latest.json` endpoint and a signing key pair: the public
key in `tauri.conf.json`, the private key only in a CI secret. The release workflow builds signed
update artifacts. A running instance checks at start and every six hours, downloads and verifies a
newer release in the background, and installs it at the next start; `/update` installs at once.
Automatic updates can be turned off in the settings. Releases are drafts until published by hand.

## Consequences

- Updates arrive without user action and without interrupting typing; only verified releases are
  installed.
- The private key must be kept safe; without it installed copies cannot be updated automatically.
- Publishing a release is a deliberate manual step after reviewing the draft.
- The installers themselves are not code-signed, so SmartScreen warns on a first manual install.

See [Release](../release.md#signed-updates).
