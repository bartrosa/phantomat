/**
 * Multi-batch Arrow IPC parity: ensure chunked Tables (e.g. produced by anywidget when
 * `Scene.add_scatter` is called more than once) are read in full instead of silently
 * discarding all but the first batch / throwing inside `extractScatterColumns`.
 */
import { Table, tableFromArrays } from "apache-arrow";
import { describe, expect, it } from "vitest";

import { flattenFloat32Column } from "../src/arrow.js";

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

describe("multi-batch Arrow Tables (anywidget repeated add_scatter)", () => {
  it("flattens chunked Float32 columns into a contiguous Float32Array", () => {
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

    const x = flattenFloat32Column(table, "x", 5);
    expect(x.length).toBe(5);
    expect(Array.from(x)).toEqual([0, 1, 2, 10, 20]);

    const y = flattenFloat32Column(table, "y", 5);
    expect(Array.from(y)).toEqual([3, 4, 5, 30, 40]);
  });

  it("returns the underlying Float32Array zero-copy for single-chunk columns", () => {
    const table = singleChunkTable({
      x: [0, 1, 2],
      y: [3, 4, 5],
      r: [1, 1, 1],
      g: [0, 0, 0],
      b: [0, 0, 0],
      a: [1, 1, 1],
      size: [4, 4, 4],
    });
    expect(table.getChild("x")!.data.length).toBe(1);
    const underlying = table.getChild("x")!.data[0]!.values as Float32Array;
    const x = flattenFloat32Column(table, "x", 3);
    expect(x).toBe(underlying);
  });

  it("throws on missing column", () => {
    const table = singleChunkTable({
      x: [0],
      y: [0],
      r: [1],
      g: [0],
      b: [0],
      a: [1],
      size: [4],
    });
    expect(() => flattenFloat32Column(table, "missing", 1)).toThrow(/missing column/);
  });

  it("throws when total chunk length disagrees with expected row count", () => {
    const table = chunkedTable([
      { x: [0, 1], y: [0, 0], r: [1, 1], g: [0, 0], b: [0, 0], a: [1, 1], size: [4, 4] },
      { x: [2], y: [0], r: [1], g: [0], b: [0], a: [1], size: [4] },
    ]);
    expect(() => flattenFloat32Column(table, "x", 99)).toThrow(/chunk total/);
  });
});
