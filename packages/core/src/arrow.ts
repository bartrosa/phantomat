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

/**
 * Returns a contiguous `Float32Array` for the named column. Single-chunk columns are zero-copy;
 * multi-chunk (chunked) columns — produced by Arrow IPC streams with more than one record batch —
 * are concatenated into a freshly allocated array so callers don't silently render only the first
 * batch.
 */
export function flattenFloat32Column(
  table: Table,
  name: string,
  expectedLength: number,
): Float32Array {
  const col = table.getChild(name);
  if (!col) {
    throw new Error(`missing column ${name}`);
  }
  const chunks = col.data;
  if (chunks.length === 0) {
    throw new Error(`column ${name}: no data chunks`);
  }
  if (chunks.length === 1) {
    const v = chunks[0]!.values as Float32Array;
    if (v.length !== expectedLength) {
      throw new Error(
        `column ${name}: expected length ${expectedLength}, got ${v.length}`,
      );
    }
    return v;
  }
  let total = 0;
  for (const d of chunks) {
    total += (d.values as Float32Array).length;
  }
  if (total !== expectedLength) {
    throw new Error(
      `column ${name}: chunk total ${total} !== numRows ${expectedLength}`,
    );
  }
  const out = new Float32Array(expectedLength);
  let off = 0;
  for (const d of chunks) {
    const v = d.values as Float32Array;
    out.set(v, off);
    off += v.length;
  }
  return out;
}

function extractScatterColumns(table: Table): { n: number; cols: Float32Array[] } {
  const n = table.numRows;
  if (n === 0) {
    throw new Error("empty Arrow table");
  }
  const cols: Float32Array[] = [];
  for (const name of COLS) {
    cols.push(flattenFloat32Column(table, name, n));
  }
  return { n, cols };
}

/**
 * Seven-column scatter layer using **wasm linear memory**: one allocation, column-major `f32`
 * blocks, then [`ScatterLayer.fromArrowPtrs`]. This avoids per-element JS↔wasm copies in the
 * legacy [`scatterFromArrowViaFloat32Arrays`] path (still available for compatibility).
 */
export async function scatterFromArrow(table: Table): Promise<ScatterLayer> {
  const wasm = await loadWasmInit();
  const { n, cols } = extractScatterColumns(table);
  const strideBytes = n * 4;
  const totalBytes = strideBytes * 7;
  const malloc = wasm.__wbindgen_export;
  const base = malloc(totalBytes, 4) >>> 0;
  if (base === 0) {
    throw new Error("wasm alloc failed for Arrow columns");
  }
  const mem = new Float32Array(wasm.memory.buffer, base, 7 * n);
  for (let i = 0; i < 7; i++) {
    mem.set(cols[i]!, i * n);
  }
  const off = (i: number) => base + i * strideBytes;
  for (let i = 0; i < 7; i++) {
    assertUsizeAlignedF32(off(i));
  }
  return ScatterLayer.fromArrowPtrs(n, off(0), off(1), off(2), off(3), off(4), off(5), off(6));
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
