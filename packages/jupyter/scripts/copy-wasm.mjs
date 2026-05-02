import { copyFileSync, mkdirSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const wasmSrc = require.resolve("phantomat-wasm/phantomat_wasm_bg.wasm");
const root = fileURLToPath(new URL("../../..", import.meta.url));
const outDir = join(root, "python/phantomat/static");
const wasmDest = join(outDir, "phantomat_wasm_bg.wasm");
mkdirSync(outDir, { recursive: true });
copyFileSync(wasmSrc, wasmDest);
