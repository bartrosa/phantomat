/**
 * End-to-end: a multi-batch (chunked) Arrow `Table` builds a `ScatterLayer` via
 * the WASM `fromArrowPtrs` path. Before the fix, `scatterFromArrow` threw on
 * any chunked table (the JS twin of the silent-data-loss bug fixed in PR #9
 * for the Python binding).
 */
import { Table, tableFromArrays } from "apache-arrow";
import { describe, expect, it } from "vitest";

import { scatterFromArrow } from "../src/arrow.js";

const COLS = ["x", "y", "r", "g", "b", "a", "size"] as const;

function singleChunkTable(values: Record<(typeof COLS)[number], number[]>): Table {
  const arrays: Record<string, Float32Array> = {};
  for (const name of COLS) arrays[name] = Float32Array.from(values[name]);
  return tableFromArrays(arrays);
}

function chunkedTable(
  parts: Array<Record<(typeof COLS)[number], number[]>>,
): Table {
  const tables = parts.map(singleChunkTable);
  return new Table(tables.flatMap((t) => t.batches));
}

describe("scatterFromArrow with chunked tables", () => {
  it("constructs a layer from a 2-chunk Table without throwing", async () => {
    const table = chunkedTable([
      {
        x: [0, 1, 2],
        y: [3, 4, 5],
        r: [1, 1, 1],
        g: [0, 0, 0],
        b: [0, 0, 0],
        a: [1, 1, 1],
        size: [4, 4, 4],
      },
      {
        x: [10, 20],
        y: [30, 40],
        r: [0, 0],
        g: [1, 1],
        b: [0, 0],
        a: [1, 1],
        size: [6, 6],
      },
    ]);
    expect(table.numRows).toBe(5);
    expect(table.getChild("x")!.data.length).toBe(2);

    const layer = await scatterFromArrow(table);
    expect(typeof layer.free).toBe("function");
    layer.free();
  });
});
