# Release

Quicklly is released from tags. Pushing a tag `vX.Y.Z` makes the release workflow verify the
version, extract the release notes from `CHANGELOG.md`, build the NSIS and MSI installers with
signed update artifacts, and attach everything to a draft GitHub release. A maintainer publishes
the draft by hand; from then on installed copies update themselves. This page describes the
versioning, the changelog, the release steps, the workflows and the update signing.

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
6. Wait for the release workflow (about ten minutes), check the draft release and its assets, and
   publish it (**Edit → Publish release**). Installed copies pick it up within hours, or at once
   with `/update`.

## Workflows

| Workflow | Trigger | Runner | Steps |
| --- | --- | --- | --- |
| [`ci.yml`](../.github/workflows/ci.yml) | Push to `main`, pull requests | `windows-latest` | `npm ci`, `npm run build`, `npm test`, `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`; a newer run cancels an older one on the same ref |
| [`release.yml`](../.github/workflows/release.yml) | Push of a tag `v*` | `windows-latest` | Verify that the tag equals the version in all three files; extract the changelog section; `tauri-apps/tauri-action` builds and drafts the release |

The release step runs with `releaseDraft: true` and `updaterJsonPreferNsis: true`, and receives
the signing key from the repository secret `TAURI_SIGNING_PRIVATE_KEY` with an empty
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (the key has no password).

## Artifacts

| Artifact | Purpose |
| --- | --- |
| `Quicklly_X.Y.Z_x64-setup.exe` | NSIS installer, per-user, installs to `%LOCALAPPDATA%\Quicklly`; used by the updater |
| `Quicklly_X.Y.Z_x64_en-US.msi` | MSI installer, per-machine, installs to `C:\Program Files\Quicklly` |
| `.sig` files | Update signatures (`bundle.createUpdaterArtifacts: true`) |
| `latest.json` | Update manifest read by installed copies |

Both installers register the `quicklly://` scheme. The installers themselves are not
code-signed, so Windows SmartScreen warns on first run; this is a known limitation documented in
the README's [Installation](../README.md#installation) section.

## Signed updates

- The updater plugin's public key and the endpoint
  `https://github.com/ruslan1us/quicklly/releases/latest/download/latest.json` are in
  [`tauri.conf.json`](../src-tauri/tauri.conf.json) under `plugins.updater`. The Windows install
  mode is `passive` (progress shown, no questions).
- The private key is held by the maintainer outside the repository and stored in the CI secret.
  It is never committed or written into documentation. Local `npm run tauri build` needs it in the
  environment; without it, `npm run tauri build -- --no-bundle` builds the executable only.
- Installed copies download and verify a newer release in the background and install it at the
  next start, or immediately on `/update`. A download superseded by a later release before it was
  installed is removed at the next check; see [Architecture](architecture.md#updates) and
  [ADR 0013](decisions/0013-signed-auto-updates-via-github-releases.md).
- Losing the private key would mean that installed copies can no longer verify new releases and
  would have to be updated by reinstalling manually.
