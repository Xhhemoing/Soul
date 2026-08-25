import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// BeadFlow is a local-first app: no proxy, no remote origin. The dev server
// binds loopback on a port that does not collide with apps/desktop (1420).
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 1430,
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
