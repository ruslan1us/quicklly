/**
 * The system the pages run on, as Node's `process.platform` names it: "win32", "darwin" (macOS)
 * or "linux". The pages are built on the system they are bundled for, and Vite puts the build's
 * platform in for `__PLATFORM__` (see `vite.config.ts`). Tests run as on Windows on any system;
 * a test for another one stubs `__PLATFORM__` before it loads a fresh copy of this module.
 */
export const platform: string = __PLATFORM__;

export const isMac = platform === "darwin";
export const isLinux = platform === "linux";

/**
 * Whether the app's own shortcuts' modifier is held: ⌘ on macOS, where Mac users expect it,
 * Ctrl elsewhere. Ctrl+C closing a window stays Ctrl everywhere, since ⌘C copies.
 */
export const primary = (e: KeyboardEvent) => (isMac ? e.metaKey : e.ctrlKey);
