import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type Theme = "default" | "defaultPlus" | "light";

/** How every window looks; transparency and scale are in percent. */
export interface Appearance {
  theme: Theme;
  transparency: number;
  scale: number;
}

/**
 * The zoom of this window's content, from the scale setting. Window sizes are set in logical
 * pixels, which are CSS pixels times this.
 */
export let zoom = 1;

export function applyTheme(theme: Theme) {
  document.documentElement.dataset.theme = theme;
}

function applyAppearance(appearance: Appearance) {
  applyTheme(appearance.theme);
  // How much of the background colour covers the blurred desktop behind the window.
  document.documentElement.style.setProperty("--opacity", `${100 - appearance.transparency}%`);
  zoom = appearance.scale / 100;
}

/**
 * Applies the saved appearance now and whenever it is changed in the settings; `onChange`
 * runs after a change, e.g. to fit the window to the new scale.
 */
export async function followTheme(onChange?: () => void) {
  await listen<Appearance>("appearance-changed", (event) => {
    applyAppearance(event.payload);
    onChange?.();
  });
  applyAppearance(await invoke<Appearance>("get_appearance"));
}
