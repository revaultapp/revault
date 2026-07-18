import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { readFileSync } from "node:fs";

const host = process.env.TAURI_DEV_HOST;
const pkg = JSON.parse(readFileSync(new URL("./package.json", import.meta.url), "utf-8"));

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [sveltekit()],

  // Build-time constant — see src/app.d.ts for the ambient declaration.
  // Read from package.json so the Settings → About version never drifts.
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },

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

  // Svelte 5 runes (and rAF-driven primitives like Tween) require the
  // "browser" condition to resolve to the client build under Vitest+jsdom.
  // Must be top-level `resolve`, not nested under `test` — Vitest doesn't
  // read `test.resolve`.
  resolve: {
    conditions: process.env.VITEST ? ["browser"] : undefined,
  },

  // Vitest configuration for frontend tests
  test: {
    environment: "jsdom",
    globals: true,
    include: ["src/**/*.test.ts"],
    setupFiles: ["./src/test-setup.ts"],
  },
}));
