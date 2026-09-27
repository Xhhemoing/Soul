import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// The WebView never loads anything it did not ship with, so there is no proxy
// here and no remote origin anywhere in this file. In development Vite binds
// loopback only; in a build the shell serves `dist/` through Tauri's own asset
// protocol.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    // WebView2 on Windows 11 is Chromium; this is the platform floor, not a
    // browser support matrix.
    target: "chrome110",
    sourcemap: false,
    emptyOutDir: true,
  },
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
    maxWorkers: 2,
    restoreMocks: true,
    unstubGlobals: true,
  },
});
