import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";
import { zoom } from "./theme";

/** Helpers for the terminal-style popup windows (settings, reader). */
export const appWindow = getCurrentWindow();

/**
 * Fits the window height to the page content, up to `maxHeight` CSS pixels; beyond that the
 * page scrolls. The window is sized in logical pixels, so CSS pixels are scaled by the zoom.
 */
export async function fitToContent(maxHeight = Infinity) {
  const height = Math.min(document.body.scrollHeight, maxHeight);
  await appWindow.setSize(new LogicalSize(window.innerWidth * zoom, height * zoom));
}

/** Shows the window, which is created hidden so it can be sized to its content first. */
export async function reveal() {
  await appWindow.center();
  await appWindow.show();
  await appWindow.setFocus();
}
