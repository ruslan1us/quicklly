import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type Theme = "default" | "defaultPlus" | "light";

export function applyTheme(theme: Theme) {
  document.documentElement.dataset.theme = theme;
}

/** Applies the saved theme now and whenever it is changed in the settings. */
export async function followTheme() {
  await listen<Theme>("theme-changed", (event) => applyTheme(event.payload));
  applyTheme(await invoke<Theme>("get_theme"));
}
