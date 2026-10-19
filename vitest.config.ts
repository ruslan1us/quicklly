import { defineConfig } from "vitest/config";

// Kept apart from vite.config.ts, which is about the app's dev server and build.
export default defineConfig({
  // The pages are tested as built for Windows on every system, so the tests don't depend on the
  // machine; a test for macOS or Linux stubs `__PLATFORM__` (see `onPlatform` in helpers.ts).
  define: {
    __PLATFORM__: JSON.stringify("win32"),
  },
  test: {
    // A browser-like DOM for every test: the windows' code and some of the pure helpers use it.
    environment: "happy-dom",
    include: ["tests/**/*.test.ts"],
    setupFiles: ["tests/setup.ts"],
    // A platform stubbed by one test is back to Windows for the next.
    unstubGlobals: true,
  },
});
