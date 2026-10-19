# Architecture Decision Records

This folder records the significant architectural and product decisions of Quicklly, one file per
decision, in the usual ADR form: **Status**, **Date**, **Context**, **Decision** and
**Consequences**. The date is when the change landed in the repository. Records describe the
decision as it was taken; they are not edited afterwards except to change the status. A decision
that is reversed or replaced gets a new record whose status names the one it supersedes, and the
old record's status becomes "Superseded by NNNN".

| No. | Decision | Date | Status |
| --- | --- | --- | --- |
| [0001](0001-tauri-2-over-electron.md) | Tauri 2 over Electron | 2026-09-26 | Accepted |
| [0002](0002-plain-markdown-files-as-the-only-storage.md) | Plain Markdown files as the only storage | 2026-09-26 | Accepted |
| [0003](0003-a-tag-is-a-file-nested-tags-are-folders.md) | A tag is a file; nested tags are folders | 2026-09-27 | Accepted |
| [0004](0004-obsidian-tasks-formats-for-due-and-done-dates.md) | Obsidian Tasks formats for due and done dates | 2026-10-13 | Accepted |
| [0005](0005-archive-done-notes-per-file-after-n-days.md) | Archive done notes per file after N days | 2026-10-13 | Accepted |
| [0006](0006-pasted-images-saved-as-files-next-to-the-notes.md) | Pasted images saved as files next to the notes, shown as labels | 2026-10-08 | Accepted; amended by 0018 |
| [0007](0007-cli-handled-in-a-fresh-process-before-tauri-starts.md) | CLI and links handled in a fresh process before Tauri starts | 2026-10-13 | Accepted; amended by 0019 |
| [0008](0008-windows-created-lazily-from-async-commands.md) | Windows created lazily from async commands; input and Pad hidden, not destroyed | 2026-09-26 | Accepted |
| [0009](0009-one-folder-per-window.md) | One folder per window in the frontend | 2026-10-14 | Accepted |
| [0010](0010-reader-changes-guarded-by-the-note-as-last-seen.md) | Reader changes guarded by the note as last seen | 2026-09-27 | Accepted |
| [0011](0011-undo-via-whole-file-snapshots.md) | Undo in the reader via whole-file snapshots | 2026-10-14 | Accepted |
| [0012](0012-settings-in-tauri-plugin-store.md) | Settings in tauri-plugin-store | 2026-09-26 | Accepted |
| [0013](0013-signed-auto-updates-via-github-releases.md) | Signed automatic updates via GitHub Releases | 2026-09-27 | Accepted; amended by 0021 |
| [0014](0014-frontend-tests-in-happy-dom-with-mocked-ipc.md) | Frontend tests in happy-dom with mocked IPC | 2026-10-13 | Accepted |
| [0015](0015-deletions-go-to-the-recycle-bin.md) | Deletions go to the Recycle Bin | 2026-10-07 | Accepted |
| [0016](0016-changelog-as-the-source-of-release-notes.md) | CHANGELOG.md as the single source of release notes | 2026-10-07 | Accepted |
| [0017](0017-watch-the-notes-folder-for-changes.md) | Watch the notes folder for changes | 2026-10-14 | Accepted |
| [0018](0018-pasted-images-let-go-of-when-the-text-starts-over.md) | Pasted images that are never saved are let go of when the text starts over | 2026-10-14 | Accepted; amends 0006 |
| [0019](0019-links-saved-by-the-running-app-on-macos.md) | Links saved by the running app on macOS | 2026-10-19 | Accepted; amends 0007 |
| [0020](0020-global-hotkey-on-wayland-via-a-desktop-shortcut.md) | Global hotkey on Wayland via a desktop shortcut | 2026-10-19 | Accepted |
| [0021](0021-deb-and-rpm-updates-only-on-request.md) | deb and rpm updates installed only on request | 2026-10-19 | Accepted; amends 0013 |

## Adding a record

1. Copy the structure of an existing record into `NNNN-short-title.md` with the next number.
2. State the context (forces and constraints), the decision (in the present tense), and the
   consequences, including the costs.
3. Set the status to "Accepted" and the date to when the change lands; if it replaces a record,
   say "Supersedes NNNN" and update the old record's status.
4. Add a row to the table above.
