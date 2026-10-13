import { defineConfig } from "vitest/config";

// Kept apart from vite.config.ts, which is about the app's dev server and build.
export default defineConfig({
  test: {
    // A browser-like DOM for every test: the windows' code and some of the pure helpers use it.
    environment: "happy-dom",
    include: ["tests/**/*.test.ts"],
    setupFiles: ["tests/setup.ts"],
  },
});
