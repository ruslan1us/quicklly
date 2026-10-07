import { invoke } from "@tauri-apps/api/core";
import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";
import { zoom } from "./theme";

/** Helpers for the terminal-style popup windows (settings, reader, help, changelog). */
export const appWindow = getCurrentWindow();

/** Width of the input window and the popups at normal size, in logical pixels. */
export const BASE_WIDTH = 640;

/**
 * Fits the window to the page content: the usual width and the content's height, up to
 * `maxHeight` CSS pixels (beyond that the page scrolls). The window is sized in logical
 * pixels, so everything is scaled by the zoom.
 */
export async function fitToContent(maxHeight = Infinity) {
  // The height depends on where lines wrap, so a window of another width (a pinned one that
  // was resized) first gets the usual width, and is measured once the page has followed.
  // The window's own size is compared, as the page may not have its zoom yet.
  const factor = await appWindow.scaleFactor();
  const width = (await appWindow.innerSize()).width / factor;
  if (Math.abs(width - BASE_WIDTH * zoom) > 1) {
    const resized = nextResize();
    await appWindow.setSize(new LogicalSize(BASE_WIDTH * zoom, window.innerHeight * zoom));
    await resized;
  }
  // The body fills the window (it is what scrolls), so measure the content inside it.
  const content = document.querySelector("main") ?? document.body;
  const height = Math.min(content.offsetHeight, maxHeight);
  await appWindow.setSize(new LogicalSize(BASE_WIDTH * zoom, height * zoom));
}

/** Resolves once the page has been laid out at a new window size (or after a short wait). */
function nextResize() {
  return new Promise<void>((resolve) => {
    window.addEventListener("resize", () => requestAnimationFrame(() => resolve()), {
      once: true,
    });
    window.setTimeout(resolve, 300);
  });
}

/**
 * Shows the window, which is created hidden so it can be sized to its content first, in the
 * middle of the screen in use.
 */
export async function reveal() {
  await invoke("center_window");
  await appWindow.show();
  await appWindow.setFocus();
}
