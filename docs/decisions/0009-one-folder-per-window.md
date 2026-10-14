# 0009. One folder per window in the frontend

- **Status:** Accepted
- **Date:** 2026-10-14

## Context

The frontend had grown to eight pages, each with its HTML, script and stylesheet, plus shared
modules, all in the repository root and a flat `src/`. It was hard to see which files belonged to
which window and which were shared.

## Decision

Each window has its own folder `src/windows/<name>/` with `index.html`, its script and its
stylesheet; everything used by more than one window lives in `src/shared/`. Vite's root is `src/`,
so pages build to `dist/windows/<name>/index.html`, and Rust addresses them with `page(name)`. No
HTML remains in the repository root.

## Consequences

- A window's code is found in one place, and shared code is explicit.
- Adding a window touches a fixed set of places: its folder, `vite.config.ts`, the capability
  file and the Rust command (see [Development](../development.md#checklist-add-a-window)).
- Tests import pages by their folder path.
