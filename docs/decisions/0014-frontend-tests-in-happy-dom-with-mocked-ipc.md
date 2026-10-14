# 0014. Frontend tests in happy-dom with mocked IPC

- **Status:** Accepted
- **Date:** 2026-10-13

## Context

Much of the behaviour is in the pages: key handling, the calendar, tag suggestions, image paste,
date conversion, the reader's footer messages and undo. Testing it by running the application is
manual and slow, and end-to-end automation of WebView2 windows would add heavy tooling.

## Decision

Test the frontend with Vitest in happy-dom. Pure modules are tested directly. Windows are tested
with their real `index.html` and script: the page body is loaded into the document, the script is
imported fresh, and the Rust side is replaced with `mockIPC` and `mockWindows` from
`@tauri-apps/api/mocks`, recording every command and answering from fixtures. A stand-in for
`document.execCommand` reproduces the browser behaviour the application relies on.

## Consequences

- Page behaviour is tested quickly in CI next to the Rust tests, with no native windows.
- Tests verify the commands a page sends and how it renders the answers, not the Rust behaviour,
  which has its own unit tests.
- Native aspects (focus, window placement, transparency, real WebView2 rendering) remain manual.

See [Testing](../testing.md).
