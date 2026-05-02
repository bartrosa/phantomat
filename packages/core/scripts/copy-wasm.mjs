import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const pkgRoot = join(__dirname, "..");
const wasmSrc = join(pkgRoot, "node_modules", "phantomat-wasm", "phantomat_wasm_bg.wasm");
const wasmDest = join(pkgRoot, "dist", "phantomat_wasm_bg.wasm");

mkdirSync(dirname(wasmDest), { recursive: true });
copyFileSync(wasmSrc, wasmDest);
