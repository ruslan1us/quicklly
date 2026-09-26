import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";

/** Helpers for the terminal-style popup windows (settings, reader). */
export const appWindow = getCurrentWindow();

/** Fits the window height to the page content, up to `maxHeight`; beyond that the page scrolls. */
export async function fitToContent(maxHeight = Infinity) {
  const height = Math.min(document.body.scrollHeight, maxHeight);
  await appWindow.setSize(new LogicalSize(window.innerWidth, height));
}

/** Shows the window, which is created hidden so it can be sized to its content first. */
export async function reveal() {
  await appWindow.center();
  await appWindow.show();
  await appWindow.setFocus();
}
