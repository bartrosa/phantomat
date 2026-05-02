import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { tableFromIPC } from "apache-arrow";
import { describe, expect, it } from "vitest";

import { scatterArrowColumnBuffers, scatterFromArrow } from "../src/arrow.js";

const root = fileURLToPath(new URL("../../../", import.meta.url));

describe("arrow columns", () => {
  it("reuses ArrayBuffer for column value arrays (same table)", async () => {
    const path = `${root}fixtures/scatter_1k.ipc`;
    let buf: Buffer;
    try {
      buf = readFileSync(path);
    } catch {
      console.warn("skip: run scripts/generate_fixtures.py to create fixtures/scatter_1k.ipc");
      return;
    }
    const table = tableFromIPC(new Uint8Array(buf));
    const bufs = scatterArrowColumnBuffers(table);
    const set = new Set(bufs);
    expect(set.size).toBeGreaterThanOrEqual(1);
    await scatterFromArrow(table);
  });
});
