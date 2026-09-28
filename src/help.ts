import { getVersion } from "@tauri-apps/api/app";
import { appWindow, fitToContent, reveal } from "./popup";
import { followTheme } from "./theme";

const header = document.querySelector<HTMLElement>("header")!;
const footer = document.querySelector<HTMLElement>("#footer")!;

/** Beyond this the help scrolls instead of the window growing. */
const MAX_HEIGHT = 560;
const SCROLL_STEP = 44;

document.addEventListener("keydown", (e) => {
  // Esc or Ctrl+C (by physical key, so any keyboard layout works) closes like in a terminal.
  if (e.key === "Escape" || (e.ctrlKey && e.code === "KeyC")) {
    e.preventDefault();
    void appWindow.close();
  } else if (e.key === "ArrowUp" || e.key === "ArrowDown") {
    e.preventDefault();
    // The body is what scrolls (see terminal.css).
    document.body.scrollBy(0, e.key === "ArrowUp" ? -SCROLL_STEP : SCROLL_STEP);
  }
});

async function load() {
  const version = await getVersion();
  header.textContent = `Quicklly · Help · v${version}`;
  footer.textContent = "↑↓ scroll · Esc close · github.com/ruslan1us/quicklly";
  await fitToContent(MAX_HEIGHT);
}

// The zoom has to be known before the window is fitted to its content.
void followTheme(() => void fitToContent(MAX_HEIGHT))
  .then(load)
  .then(reveal);
