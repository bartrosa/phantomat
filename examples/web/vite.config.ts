import { defineConfig } from "vite";

// Preview serves the static `dist/` build on :4173 (for Playwright e2e).
export default defineConfig({
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
  preview: {
    port: 4173,
    strictPort: true,
  },
  server: {
    port: 5173,
  },
});
