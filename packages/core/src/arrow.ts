import { parseRecordBatch } from "arrow-js-ffi";
import { Table } from "apache-arrow";
import type { InitOutput } from "phantomat-wasm";
import init, { ScatterLayer } from "phantomat-wasm";

export { parseRecordBatch };

const COLS = ["x", "y", "r", "g", "b", "a", "size"] as const;

/** Browser: fetch default wasm. Node/Vitest: resolve wasm via `require.resolve` (pnpm-safe). */
async function loadWasmInit(): Promise<InitOutput> {
  const node =
    typeof globalThis.process !== "undefined" &&
    Boolean(globalThis.process.versions?.node);
  if (node) {
    const { readFileSync } = await import("node:fs");
    const { createRequire } = await import("node:module");
    const require = createRequire(import.meta.url);
    const wasmPath = require.resolve("phantomat-wasm/phantomat_wasm_bg.wasm");
    return (await init({
      module_or_path: readFileSync(wasmPath),
    })) as InitOutput;
  }
  return (await init()) as InitOutput;
}

function assertUsizeAlignedF32(ptr: number): void {
  // usize is used as byte offset; f32 requires 4-byte alignment.
  if ((ptr >>> 0) % 4 !== 0) {
    throw new Error("column pointer not 4-byte aligned for f32");
  }
}

function extractScatterColumns(table: Table): { n: number; cols: Float32Array[] } {
  const n = table.numRows;
  if (n === 0) {
    throw new Error("empty Arrow table");
  }
  const cols: Float32Array[] = [];
  for (const name of COLS) {
    const col = table.getChild(name);
    if (!col) {
      throw new Error(`missing column ${name}`);
    }
    const v = col.data[0]!.values as Float32Array;
    if (v.length !== n) {
      throw new Error(`column ${name}: expected length ${n}, got ${v.length}`);
    }
    cols.push(v);
  }
  return { n, cols };
}

/**
 * Seven-column scatter layer using **wasm linear memory**: one allocation, column-major `f32`
 * blocks, then [`ScatterLayer.fromArrowBlock`]. This avoids per-element JS↔wasm copies in the
 * legacy [`scatterFromArrowViaFloat32Arrays`] path (still available for compatibility).
 *
 * **Ownership:** the wasm-malloc block is handed off to the returned [`ScatterLayer`] which
 * frees it when dropped (via Rust `Drop`). Repeated `scatterFromArrow` calls — e.g. each
 * Jupyter widget update — therefore do **not** leak the previous allocation.
 */
export async function scatterFromArrow(table: Table): Promise<ScatterLayer> {
  const wasm = await loadWasmInit();
  const { n, cols } = extractScatterColumns(table);
  const strideBytes = n * 4;
  const totalBytes = strideBytes * 7;
  const malloc = wasm.__wbindgen_export;
  const align = 4;
  const base = malloc(totalBytes, align) >>> 0;
  if (base === 0) {
    throw new Error("wasm alloc failed for Arrow columns");
  }
  // After malloc, refresh the Float32 view in case linear memory was grown.
  const mem = new Float32Array(wasm.memory.buffer, base, 7 * n);
  for (let i = 0; i < 7; i++) {
    mem.set(cols[i]!, i * n);
  }
  for (let i = 0; i < 7; i++) {
    assertUsizeAlignedF32(base + i * strideBytes);
  }
  // ScatterLayer takes ownership of the [base, base+totalBytes) block and frees it on drop.
  return ScatterLayer.fromArrowBlock(n, base, totalBytes, align);
}

/** Legacy path: copies via wasm-bindgen [`Float32Array`] handles (kept for compatibility). */
export async function scatterFromArrowViaFloat32Arrays(table: Table): Promise<ScatterLayer> {
  await loadWasmInit();
  const { cols } = extractScatterColumns(table);
  return ScatterLayer.fromArrowFloat32Arrays(
    cols[0]!,
    cols[1]!,
    cols[2]!,
    cols[3]!,
    cols[4]!,
    cols[5]!,
    cols[6]!,
  );
}

/** Float32 column value buffers backing the Arrow table (JS heap / IPC); useful for reuse checks. */
export function scatterArrowColumnBuffers(table: Table): ArrayBuffer[] {
  const out: ArrayBuffer[] = [];
  for (const name of COLS) {
    const col = table.getChild(name);
    if (!col) throw new Error(`missing column ${name}`);
    const ta = col.data[0]!.values as Float32Array;
    out.push(
      ta.buffer.slice(ta.byteOffset, ta.byteOffset + ta.byteLength) as ArrayBuffer,
    );
  }
  return out;
}
