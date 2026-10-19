/**
 * The system the pages run on, as Node's `process.platform` names it: "win32", "darwin" (macOS)
 * or "linux". The pages are built on the system they are bundled for, and Vite puts the build's
 * platform in for `__PLATFORM__` (see `vite.config.ts`). Tests run as on Windows on any system;
 * a test for another one stubs `__PLATFORM__` before it loads a fresh copy of this module.
 */
export const platform: string = __PLATFORM__;

export const isWindows = platform === "win32";
export const isMac = platform === "darwin";
export const isLinux = platform === "linux";

/**
 * Whether the app's own shortcuts' modifier is held: ⌘ on macOS, where Mac users expect it,
 * Ctrl elsewhere. Ctrl+C closing a window stays Ctrl everywhere, since ⌘C copies.
 */
export const primary = (e: KeyboardEvent) => (isMac ? e.metaKey : e.ctrlKey);

/**
 * What the hints and the help call keys and things on this system. A modifier is written with
 * the key it goes with, as `${names.mod}P`: "Ctrl+P", or "⌘P" on macOS, where the app's own
 * shortcuts take ⌘ (`mod`) while Ctrl+C, which closes, stays ⌃ (`ctrl`).
 */
export const names = {
  ...(isMac
    ? { mod: "⌘", ctrl: "⌃", shift: "⇧", del: "⌫" }
    : { mod: "Ctrl+", ctrl: "Ctrl+", shift: "Shift+", del: "Del" }),
  /** Where deleted files go. */
  trash: isWindows ? "Recycle Bin" : "Trash",
  /** The program run from a terminal. */
  cli: isWindows ? "quicklly.exe" : "quicklly",
};

/** Puts this system's names into a page's own text: `<span data-name="mod">Ctrl+</span>P`. */
export function nameThings(root: ParentNode = document) {
  for (const el of root.querySelectorAll<HTMLElement>("[data-name]")) {
    el.textContent = names[el.dataset.name as keyof typeof names];
  }
}
