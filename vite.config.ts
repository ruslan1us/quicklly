import { defineConfig } from "vite";
// @ts-expect-error type error without @types/node package
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
  // The pages live in src/windows/<name>/index.html and are served and built from src/,
  // so the app opens them as windows/<name>/index.html.
  root: "src",
  // The dev server looks for dependencies from the pages too, but resolves them from the root
  // (src/), unlike the build inputs below.
  optimizeDeps: {
    entries: ["windows/*/index.html"],
  },
  // Each window is its own page.
  build: {
    outDir: "../dist",
    emptyOutDir: true,
    rolldownOptions: {
      input: {
        input: "src/windows/input/index.html",
        settings: "src/windows/settings/index.html",
        reader: "src/windows/reader/index.html",
        pad: "src/windows/pad/index.html",
        help: "src/windows/help/index.html",
        changelog: "src/windows/changelog/index.html",
        reminders: "src/windows/reminders/index.html",
        done: "src/windows/done/index.html",
      },
    },
  },
}));
