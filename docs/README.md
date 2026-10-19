# Quicklly documentation

This folder holds the internal documentation of Quicklly: how the application is built, why it is
built that way, and the conventions that keep it consistent. It is written for coding agents and
human contributors who change the code. The user-facing guide (keys, commands, settings) is the
root [README](../README.md); these pages link to its sections instead of repeating them. Coding
agents start from [`AGENTS.md`](../AGENTS.md) in the repository root, which points here.

## Pages

| Page | Covers |
| --- | --- |
| [Overview](overview.md) | Purpose, product principles, scope, the platforms and how they differ, non-goals, direction |
| [Architecture](architecture.md) | Process model, windows and their life cycle, IPC commands and events, managed state, background threads, single instance, deep links, updater |
| [Data format](data-format.md) | The notes folder, the note line format, tags, dates, images, reserved names, `settings.json` |
| [Windows](windows/README.md) | One page per window: [input](windows/input.md), [Pad](windows/pad.md), [reader](windows/reader.md), [reminders](windows/reminders.md), [done](windows/done.md), [settings](windows/settings.md), [help and changelog](windows/help-and-changelog.md) |
| [Backend](backend.md) | The Rust modules, error handling, file writes, undo snapshots |
| [Frontend](frontend.md) | The Vite multi-page setup, shared modules, styling and editing conventions |
| [CLI and links](cli-and-links.md) | `quicklly add …` and `quicklly://add?text=…` |
| [Development](development.md) | Prerequisites on Windows, macOS and Linux, commands, checks, conventions, known pitfalls, checklists for adding a window, a command or a setting |
| [Testing](testing.md) | Vitest with happy-dom and mocked IPC, Rust unit tests, manual verification on each system |
| [Release](release.md) | Versioning, changelog, CI workflows, signed updates, the installers and packages for each system |
| [Decisions](decisions/README.md) | Architecture Decision Records |

## Reading order before changing code

1. [Overview](overview.md): the principles that rule out many otherwise reasonable changes.
2. [Architecture](architecture.md): how windows, commands and events fit together.
3. [Data format](data-format.md): required before touching anything that reads or writes notes.
   The files belong to the user and are also read by other tools.
4. The page of each part being changed: a [window](windows/README.md), the
   [backend](backend.md) or the [frontend](frontend.md).
5. [Development](development.md): conventions, the known pitfalls and the checklists.
6. [Decisions](decisions/README.md): a change that contradicts a recorded decision needs a new
   record that supersedes it.

## Keeping the documentation current

- A change in behaviour, structure or conventions updates the pages that describe it in the same
  commit as the code.
- User-visible behaviour is described in the root [README](../README.md), the in-app help
  ([`src/windows/help/index.html`](../src/windows/help/index.html)) and
  [`CHANGELOG.md`](../CHANGELOG.md); these pages describe the implementation and link there.
- A new architectural decision, or a reversal of an old one, gets a new numbered record in
  [`decisions/`](decisions/README.md). Accepted records are not rewritten; they are superseded.
- Statements in these pages are verified against the code. Where the code and a page disagree,
  the code is authoritative and the page is corrected.
- Links between pages and to source files are relative, so they work on GitHub and in a local
  checkout.
