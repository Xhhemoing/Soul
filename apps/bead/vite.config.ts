import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// BeadFlow is a local-first app: no proxy, no remote origin. E0-8 in the
// round-1 CI review pins 1520 so this dev server can run beside apps/desktop,
// which pins 1420.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 1520,
    strictPort: true,
  },
  build: {
    target: "chrome110",
    sourcemap: true,
    emptyOutDir: true,
  },
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
    restoreMocks: true,
    unstubGlobals: true,
  },
});
