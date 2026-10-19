# Agent guide

Quicklly is a hotkey notepad for Windows, macOS and Linux built with Tauri 2 (Rust backend,
Vite/TypeScript frontend). This file is the entry point for coding agents working on the repository.

## Before changing code

Read the project documentation in [`docs/`](docs/README.md), starting with its index. At a
minimum, read:

- [`docs/overview.md`](docs/overview.md) for the product principles and scope;
- [`docs/architecture.md`](docs/architecture.md) for how windows, commands and events fit together;
- [`docs/data-format.md`](docs/data-format.md) before touching anything that reads or writes notes;
- the page for each part being changed (a window in [`docs/windows/`](docs/windows/), the
  [backend](docs/backend.md) or [frontend](docs/frontend.md) modules);
- [`docs/development.md`](docs/development.md) for conventions and known pitfalls.

Past architectural decisions are recorded in [`docs/decisions/`](docs/decisions/README.md). A
change that goes against one of them needs a new decision record, not a silent deviation.

## Working rules

- Keep changes small: one logical change per commit, described with
  [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `docs:`,
  `refactor:`, `test:`, `ci:`, `chore:`).
- Everything in the repository is written in English: code comments, documentation, commit
  messages.
- Match the surrounding code: naming, comment density and idiom.
- Prefer the simplest implementation. Avoid input restrictions and heuristics that guess the
  user's intent.
- User-visible changes get a line under `## [Unreleased]` in [`CHANGELOG.md`](CHANGELOG.md) and,
  where relevant, an update to [`README.md`](README.md) and the in-app help.
- Keep `docs/` current: a change in behaviour, structure or conventions updates the pages that
  describe it in the same change.

## Checks

All of these must pass before a change is committed:

```sh
npm run build
npm test
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

CI runs them on Windows, macOS and Linux. Rust code behind `cfg(target_os = "macos")` or
`cfg(target_os = "linux")` is compiled only there, so such a change is not checked until CI has
run; see [`docs/development.md`](docs/development.md#prerequisites).

Tests must not touch real user data, the real Recycle Bin or Trash, or system settings; see
[`docs/testing.md`](docs/testing.md).
