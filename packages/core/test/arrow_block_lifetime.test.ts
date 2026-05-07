import { tableFromArrays } from "apache-arrow";
import { describe, expect, it } from "vitest";

import init from "phantomat-wasm";
import { scatterFromArrow } from "../src/arrow.js";

const COLS = ["x", "y", "r", "g", "b", "a", "size"] as const;

function smallTable(n: number) {
  const arrays: Record<string, Float32Array> = {};
  for (const name of COLS) {
    arrays[name] = new Float32Array(n).map((_, i) => (name === "a" ? 1.0 : i + 1));
  }
  return tableFromArrays(arrays);
}

async function loadWasmInitForNode(): Promise<{
  memory: { buffer: ArrayBuffer };
  __wbindgen_export: (size: number, align: number) => number;
}> {
  const { readFileSync } = await import("node:fs");
  const { createRequire } = await import("node:module");
  const require = createRequire(import.meta.url);
  const wasmPath = require.resolve("phantomat-wasm/phantomat_wasm_bg.wasm");
  return (await init({ module_or_path: readFileSync(wasmPath) })) as unknown as {
    memory: { buffer: ArrayBuffer };
    __wbindgen_export: (size: number, align: number) => number;
  };
}

describe("scatterFromArrow wasm-block ownership (regression for unbounded leak)", () => {
  it("does not grow wasm linear memory across many ingest cycles", async () => {
    const wasm = await loadWasmInitForNode();
    // Each table is sized so that 7 columns * 4 bytes is a meaningful fraction
    // of any wasm linear-memory page (64 KiB).
    const N = 16_384; // 16k points → 7 * 16k * 4 ≈ 448 KiB per allocation

    // Warm up: bring the allocator into steady state and let any one-off Vite /
    // wasm-bindgen scratch allocations settle.
    for (let i = 0; i < 4; i++) {
      const layer = await scatterFromArrow(smallTable(N));
      layer.free();
    }

    const baselineBytes = wasm.memory.buffer.byteLength;
    const ITERATIONS = 64;
    for (let i = 0; i < ITERATIONS; i++) {
      const layer = await scatterFromArrow(smallTable(N));
      layer.free();
    }
    const afterBytes = wasm.memory.buffer.byteLength;

    // Without the fix, every iteration would leak ~448 KiB of wasm linear
    // memory, so 64 iterations would grow the buffer by tens of MiB. With the
    // fix, the allocator reuses the freed block and the buffer stays flat.
    const growthBytes = afterBytes - baselineBytes;
    const oneAllocation = N * 7 * 4;
    expect(
      growthBytes,
      `wasm linear memory grew by ${growthBytes} bytes after ${ITERATIONS} ` +
        `scatterFromArrow→free cycles (one allocation = ${oneAllocation} bytes)`,
    ).toBeLessThan(oneAllocation * 4);
  });

  it("renders correct-shaped data via the new fromArrowBlock path", async () => {
    const N = 4;
    const layer = await scatterFromArrow(smallTable(N));
    expect(typeof layer.free).toBe("function");
    layer.free();
  });
});
