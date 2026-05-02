import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import dts from "vite-plugin-dts";

const __dirname = fileURLToPath(new URL(".", import.meta.url));

export default defineConfig({
  plugins: [
    dts({
      insertTypesEntry: true,
      exclude: [
        "vite.config.ts",
        "vitest.config.ts",
        "**/test/**",
        "**/test-d/**",
      ],
    }),
  ],
  build: {
    lib: {
      entry: resolve(__dirname, "src/index.ts"),
      name: "Phantomat",
      formats: ["es", "cjs", "umd"],
      fileName: (format) => {
        if (format === "es") return "index.es.js";
        if (format === "cjs") return "index.cjs";
        return "index.umd.js";
      },
    },
    rollupOptions: {
      external: ["phantomat-wasm"],
      output: {
        globals: {
          "phantomat-wasm": "PhantomatWasm",
        },
      },
    },
  },
});
