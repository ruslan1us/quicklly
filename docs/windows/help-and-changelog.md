# Help and changelog

Two read-only popups document the application from inside it. `/help` shows the keys, the
commands and short explanations of tags, dates, reminders, the done journal, the archive, the
reader, the Pad and saving from scripts, with the running version in the header. `/changelog`
shows the release history from `CHANGELOG.md`, embedded at build time. Both are opened from the
input window and closed with <kbd>Esc</kbd>.

## Files

| File | Role |
| --- | --- |
| [`src/windows/help/index.html`](../../src/windows/help/index.html) | The help content, as static HTML |
| [`src/windows/help/help.ts`](../../src/windows/help/help.ts) | Version in the header and footer, keys, fit and reveal |
| [`src/windows/help/help.css`](../../src/windows/help/help.css) | Layout; imports `terminal.css` |
| [`src/windows/changelog/index.html`](../../src/windows/changelog/index.html) | Header, release container, footer |
| [`src/windows/changelog/changelog.ts`](../../src/windows/changelog/changelog.ts) | Parses and renders `CHANGELOG.md` |
| [`src/windows/changelog/changelog.css`](../../src/windows/changelog/changelog.css) | Layout; imports `terminal.css` |
| [`CHANGELOG.md`](../../CHANGELOG.md) | Source of the changelog window and of the GitHub release notes |

## Help

- The content is written in `index.html`. It must be updated together with user-visible changes,
  like the root README.
- The header reads `Quicklly · Help · v<version>` from `getVersion()`; the footer names the
  repository.
- Some values in the text are the defaults (for example "after 7 days" for the archive); the help
  does not read the settings.

## Changelog

- `changelog.ts` imports `../../../CHANGELOG.md?raw`, so the text is bundled into the build; the
  window shows the changelog of the running version without network access.
- `parseChangelog` reads the [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) structure:
  `## [X.Y.Z] - YYYY-MM-DD` starts a release (shown as `X.Y.Z · YYYY-MM-DD`), `### Group` starts a
  group, `- ` starts an entry, and indented lines continue the previous entry. The introduction
  before the first release and the link definitions at the end are skipped. Backticks are removed;
  other inline Markdown stays as text.
- `## [Unreleased]` is shown only when it has at least one entry.
- The format is also what the release workflow extracts for the GitHub release; see
  [Release](../release.md#changelog).

## Keys

| Key | Action |
| --- | --- |
| <kbd>↑</kbd> <kbd>↓</kbd> | Scroll |
| <kbd>Esc</kbd>, <kbd>Ctrl</kbd>+<kbd>C</kbd> | Close |

Both windows fit their content up to 560 CSS pixels and then scroll.
