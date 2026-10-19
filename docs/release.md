# Release

Quicklly is released from tags. Pushing a tag `vX.Y.Z` makes the release workflow verify the
version, extract the release notes from `CHANGELOG.md`, build the Windows installers, the macOS
app and the Linux packages with signed update artifacts, and attach everything to a draft GitHub
release. A maintainer publishes the draft by hand; from then on installed copies update
themselves. This page describes the versioning, the changelog, the release steps, the workflows
and the update signing.

## Versioning

Versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Before 1.0, a minor
version (`0.8.0`) groups several features; patch versions (`0.8.1`) carry fixes or a smaller set
of features. Commits stay small regardless of how many go into a version.

The version is kept in three files, which must agree:

| File | Field |
| --- | --- |
| [`package.json`](../package.json) | `version` (set with `npm version X.Y.Z --no-git-tag-version`) |
| [`src-tauri/Cargo.toml`](../src-tauri/Cargo.toml) | `[package] version` (`cargo check` then updates `Cargo.lock`) |
| [`src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json) | `version`; this one names the installers and is compared by the updater |

## Changelog

[`CHANGELOG.md`](../CHANGELOG.md) follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/):
newest first, a `## [Unreleased]` section at the top, `### Added`, `### Changed`, `### Fixed`
groups, one user-facing sentence per entry, and link definitions at the bottom. Every user-visible
change adds an entry under `[Unreleased]` in the same commit.

The changelog has two consumers, which is why its structure matters:

- The release workflow uses the `## [X.Y.Z]` section as the body of the GitHub release, and fails
  if the section is missing or empty.
- The `/changelog` window renders it from the bundled file; see
  [Help and changelog](windows/help-and-changelog.md#changelog).

See [ADR 0016](decisions/0016-changelog-as-the-source-of-release-notes.md).

## Steps

1. In `CHANGELOG.md`, rename `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD`, add a new empty
   `## [Unreleased]` above it, and update the links at the bottom:
   `[Unreleased]: …/compare/vX.Y.Z...HEAD` and `[X.Y.Z]: …/compare/vPREVIOUS...vX.Y.Z`.
2. Set the version in the three files and run `cargo check` in `src-tauri/` to update
   `Cargo.lock`.
3. Run the [checks](development.md#checks-before-every-commit).
4. Commit as `chore: release vX.Y.Z` and create an annotated tag:
   `git tag -a vX.Y.Z -m "Quicklly vX.Y.Z"`.
5. Push the commit and the tag. Agents do this only on an explicit instruction from the
   maintainer.
6. Wait for the release workflow (about half an hour: the Windows, macOS and Linux jobs run one
   after another), check the draft release and its assets, and that `latest.json` has the
   `windows-x86_64`, `darwin-aarch64`, `darwin-x86_64` and `linux-x86_64` entries, and publish it
   (**Edit → Publish release**). Installed copies pick it up within hours, or at once with
   `/update`.

## Workflows

| Workflow | Trigger | Runner | Steps |
| --- | --- | --- | --- |
| [`ci.yml`](../.github/workflows/ci.yml) | Push to any branch, pull requests | Matrix: `windows-latest`, `macos-latest`, `ubuntu-24.04` (`fail-fast: false`) | On Ubuntu, `apt-get install` the Tauri Linux libraries (`libwebkit2gtk-4.1-dev`, `libdbus-1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`); then `npm ci`, `npm run build`, `npm test`, `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`; a newer run cancels an older one on the same ref |
| [`release.yml`](../.github/workflows/release.yml) | Push of a tag `v*` | Three jobs one after another: `windows-latest`, then `macos-latest` (`needs: release`), then `ubuntu-24.04` (`needs: [release, macos]`) | Windows: verify that the tag equals the version in all three files, extract the changelog section, and `tauri-apps/tauri-action` builds the installers and drafts the release. macOS: a universal build (`--target universal-apple-darwin`, Rust targets `aarch64-apple-darwin` and `x86_64-apple-darwin`). Linux: `apt-get install` the libraries CI installs plus `xdg-utils`, then the AppImage, deb and rpm. Each later job attaches its files to the same draft |

Every tauri-action step runs with `releaseDraft: true` and the same `tagName`, and receives the
signing key from the repository secret `TAURI_SIGNING_PRIVATE_KEY` with an empty
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (the key has no password). The Windows step also sets
`updaterJsonPreferNsis: true`. The macOS signature is ad hoc, from the
[macOS bundle settings](#macos-bundle), so no Apple secrets are needed; Linux packages have no
code signature of their own.

The Windows job creates the draft. The later jobs find it by its tag and upload to it; tauri-action
leaves the name and body of an existing draft as they are. Each tauri-action run also downloads
the draft's `latest.json`, adds the entries for its own platform and uploads it again, which is why
the jobs run one after another rather than in parallel: two runs at once could each upload a file
without the other's entries. A later job failing leaves a draft without that platform; re-run
the failed jobs before publishing.

The release notes come from `CHANGELOG.md` once, in the Windows job: they become the body of the
draft and the `notes` of `latest.json`. The job passes them to the later jobs as its `notes`
output, because each run writes `notes` into `latest.json` anew. The version check also runs only
in the Windows job; the later jobs don't start if it fails.

Because these jobs see the signing key that every installed copy trusts, the actions in
`release.yml` are pinned to a commit (`uses: owner/action@<sha> # vX.Y.Z`) rather than to a tag
or branch that could be moved. To update one, look up the commit of the new version
(`git ls-remote https://github.com/<owner>/<action> refs/tags/vX.Y.Z refs/tags/vX.Y.Z^{}`; for an
annotated tag the `^{}` line is the commit) and replace the SHA and the comment.

## Artifacts

| Artifact | Purpose |
| --- | --- |
| `Quicklly_X.Y.Z_x64-setup.exe` | NSIS installer, per-user, installs to `%LOCALAPPDATA%\Quicklly`; used by the updater |
| `Quicklly_X.Y.Z_x64_en-US.msi` | MSI installer, per-machine, installs to `C:\Program Files\Quicklly` |
| `Quicklly_X.Y.Z_universal.dmg` | macOS disk image with the universal app (Apple silicon and Intel) |
| `Quicklly_X.Y.Z_universal.app.tar.gz` | The macOS app packed for the updater |
| `Quicklly_X.Y.Z_amd64.AppImage` | Linux, a single file that runs without installing; used by the updater by default |
| `Quicklly_X.Y.Z_amd64.deb` | Linux package for Debian, Ubuntu and their derivatives |
| `Quicklly-X.Y.Z-1.x86_64.rpm` | Linux package for Fedora, openSUSE and their derivatives |
| `.sig` files | Update signatures (`bundle.createUpdaterArtifacts: true`), one for the NSIS and MSI installers, the `.app.tar.gz`, the AppImage, the deb and the rpm |
| `latest.json` | Update manifest read by installed copies |

`latest.json` names each download under one or more platform keys: `windows-x86_64` (the NSIS
installer) with `windows-x86_64-nsis` and `-msi`; `darwin-aarch64`, `darwin-x86_64` and
`darwin-universal`, with `-app` variants, all pointing to the universal `.app.tar.gz`;
`linux-x86_64` (the AppImage) with `linux-x86_64-appimage`, `-deb` and `-rpm`. The updater looks up
`<os>-<arch>-<bundle type>` first and then `<os>-<arch>`, so a deb install updates from the deb and
an rpm install from the rpm.

The macOS app needs macOS 13.1 or later (`minimumSystemVersion`). The Linux files are built on
Ubuntu 24.04, so the AppImage needs glibc 2.39 or later (Ubuntu 24.04, Debian 13, Fedora 40 and
newer).

Both Windows installers register the `quicklly://` scheme. They are not code-signed, so Windows
SmartScreen warns on first run; this is a known limitation documented in the README's
[Installation](../README.md#installation) section.

## Bundle settings

The bundle settings shared by every platform (icons, updater artifacts) are in
[`tauri.conf.json`](../src-tauri/tauri.conf.json). Settings for one platform go in a platform file
next to it, which the Tauri CLI and `tauri-build` merge over the base file as a
[JSON Merge Patch](https://datatracker.ietf.org/doc/html/rfc7396) only when building for that
platform: a Windows build never reads them. In a merge patch an object is merged key by key, but
an array or a string replaces the base value.

[`tauri.linux.conf.json`](../src-tauri/tauri.linux.conf.json) holds the Linux packages' settings:

| Key | Value | Purpose |
| --- | --- | --- |
| `bundle.targets` | `appimage`, `deb`, `rpm` | The three Linux formats (the same set as `"all"` on Linux, named explicitly) |
| `bundle.category` | `Utility` | `Categories=Utility;` in the `.desktop` file |
| `bundle.shortDescription`, `bundle.longDescription` | A one-line summary and a paragraph | Package summary and description, and the `.desktop` `Comment`. Without them the bundler uses the `description` from [`Cargo.toml`](../src-tauri/Cargo.toml), "Hotkey mini-notepad", which the Windows installers also use in their metadata |
| `bundle.linux.deb.depends`, `bundle.linux.rpm.depends` | `xdg-utils`, `desktop-file-utils` | `xdg-mime` and `update-desktop-database` for the [registration of `quicklly://`](cli-and-links.md#registration-of-the-scheme) at start, `xdg-open` for opening the notes folder and images |
| `bundle.linux.deb.desktopTemplate`, `bundle.linux.rpm.desktopTemplate` | [`linux/quicklly.desktop`](../src-tauri/linux/quicklly.desktop) | The bundler's default `.desktop` template with `%u` added to `Exec`, so the packaged launcher passes a `quicklly://` link on. The AppImage is built from the deb's files and gets the same `.desktop` file |

The CLI adds the rest of the package dependencies itself: WebKitGTK and GTK, and, because the
`tauri` crate has the `tray-icon` feature, the appindicator library the tray loads at run time
(`libayatana-appindicator3-1` for deb, `libayatana-appindicator3.so.1()(64bit)` for rpm; without
it the tray panics at start). It finds that library on the build machine with `pkg-config`, so the
build machine needs `libayatana-appindicator3-dev`, as CI installs. Adding it again in `depends`
would list it twice.

## macOS bundle

The macOS settings are in [`tauri.macos.conf.json`](../src-tauri/tauri.macos.conf.json), which
Tauri merges over `tauri.conf.json` (JSON Merge Patch) only when building for macOS; Windows
builds don't read it. The release workflow builds it as one universal app.

| Setting | Value | Why |
| --- | --- | --- |
| `identifier` | `com.quicklly.desktop` | The shared identifier ends in `.app`, so Finder would show the data and cache folders named after it (`~/Library/Application Support/com.quicklly.app`) as broken applications. Windows keeps `com.quicklly.app`: it names the `%APPDATA%` folder of every installed copy. |
| `bundle.targets` | `app`, `dmg` | The DMG installs the app; with `createUpdaterArtifacts` the bundle is also packed as `Quicklly.app.tar.gz` with a `.sig` for the updater. |
| `bundle.macOS.minimumSystemVersion` | `13.1` | The pages rely on the system WebKit: `color-mix()` needs Safari 16.2, which came with macOS 13.1. |
| `bundle.macOS.signingIdentity` | `-` | Ad-hoc signing: there is no Apple Developer ID and no notarization, so Gatekeeper blocks the first start until **Open Anyway** in System Settings → Privacy & Security. Updates are verified by the updater's own signature. |

[`Info.plist`](../src-tauri/Info.plist) next to the config is merged into the bundle's
`Info.plist` by the bundler:

- `LSUIElement`: Quicklly is a menu-bar app from the moment it starts, with no Dock icon, menu bar
  or ⌘-Tab entry. It applies to the bundled app only; `npm run tauri dev` runs without a bundle.
- `NSDocumentsFolderUsageDescription`: the text of the prompt for access to `~/Documents`, where
  the default notes folder is.

The `quicklly://` scheme (`CFBundleURLTypes`) is not in this file: the bundler writes it from
`plugins.deep-link.desktop.schemes`.

## Signed updates

- The updater plugin's public key and the endpoint
  `https://github.com/ruslan1us/quicklly/releases/latest/download/latest.json` are in
  [`tauri.conf.json`](../src-tauri/tauri.conf.json) under `plugins.updater`. The Windows install
  mode is `passive` (progress shown, no questions).
- The private key is held by the maintainer outside the repository and stored in the CI secret.
  It is never committed or written into documentation. Local `npm run tauri build` needs it in the
  environment; without it, `npm run tauri build -- --no-bundle` builds the executable only.
- Installed copies download and verify a newer release in the background and install it at the
  next start, or immediately on `/update`. A deb or rpm install waits for `/update`: the plugin
  installs the package through `pkexec`, which asks for the root password, so it is never started
  at login ([ADR 0021](decisions/0021-deb-and-rpm-updates-only-on-request.md)). The signature is
  verified only when downloading, so the download is saved through a temporary file renamed into
  place: the stored file is complete or absent, never cut short. A download superseded by a later release before it was
  installed is removed at the next check; see [Architecture](architecture.md#updates) and
  [ADR 0013](decisions/0013-signed-auto-updates-via-github-releases.md).
- Losing the private key would mean that installed copies can no longer verify new releases and
  would have to be updated by reinstalling manually.
