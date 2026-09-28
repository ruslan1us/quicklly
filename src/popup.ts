import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";
import { zoom } from "./theme";

/** Helpers for the terminal-style popup windows (settings, reader). */
export const appWindow = getCurrentWindow();

/** Width of the input window and the popups at normal size, in logical pixels. */
export const BASE_WIDTH = 640;

/**
 * Fits the window to the page content: the usual width and the content's height, up to
 * `maxHeight` CSS pixels (beyond that the page scrolls). The window is sized in logical
 * pixels, so everything is scaled by the zoom.
 */
export async function fitToContent(maxHeight = Infinity) {
  const height = Math.min(document.body.scrollHeight, maxHeight);
  await appWindow.setSize(new LogicalSize(BASE_WIDTH * zoom, height * zoom));
}

/** Shows the window, which is created hidden so it can be sized to its content first. */
export async function reveal() {
  await appWindow.center();
  await appWindow.show();
  await appWindow.setFocus();
}
