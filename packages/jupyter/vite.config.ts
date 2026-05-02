import { mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

const __dirname = dirname(fileURLToPath(import.meta.url));
const outDir = resolve(__dirname, "../../python/phantomat/static");

export default defineConfig({
  assetsInclude: ["**/*.wasm"],
  plugins: [
    {
      name: "ensure-static-out",
      buildStart() {
        mkdirSync(outDir, { recursive: true });
      },
    },
  ],
  build: {
    lib: {
      entry: resolve(__dirname, "src/widget.ts"),
      name: "PhantomatWidget",
      formats: ["es"],
      fileName: () => "widget.js",
    },
    outDir,
    emptyOutDir: false,
    rollupOptions: {
      output: {
        inlineDynamicImports: true,
        entryFileNames: "widget.js",
        assetFileNames: (info) => {
          const name = info.names?.[0] ?? info.name ?? "";
          if (String(name).endsWith(".wasm")) return "phantomat_wasm_bg.wasm";
          return "[name][extname]";
        },
      },
    },
  },
});
