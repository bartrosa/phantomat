import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { tableFromIPC } from "apache-arrow";
import { describe, expect, it } from "vitest";

import {
  scatterArrowColumnBuffers,
  scatterFromArrow,
  scatterFromArrowViaFloat32Arrays,
} from "../src/arrow.js";

const root = fileURLToPath(new URL("../../../", import.meta.url));

const COLS = ["x", "y", "r", "g", "b", "a", "size"] as const;

function fixtureColumnSha256(table: import("apache-arrow").Table): string {
  const h = createHash("sha256");
  for (const name of COLS) {
    const col = table.getChild(name);
    if (!col) throw new Error(`missing ${name}`);
    const v = col.data[0]!.values as Float32Array;
    h.update(Buffer.from(v.buffer, v.byteOffset, v.byteLength));
  }
  return h.digest("hex");
}

describe("differential Arrow fixture", () => {
  it("stable SHA256 of scatter_1k IPC columns (seed 42)", async () => {
    const path = `${root}fixtures/scatter_1k.ipc`;
    let buf: Buffer;
    try {
      buf = readFileSync(path);
    } catch {
      console.warn("skip: run scripts/generate_fixtures.py");
      return;
    }
    const table = tableFromIPC(new Uint8Array(buf));
    const digest = fixtureColumnSha256(table);
    // Generated once from `python scripts/generate_fixtures.py` + this hash routine.
    expect(digest).toBe(
      "5fa623e65e7338e60f7754d87cc370a73cfd2afdcbf4fc4f5be584565db4eda8",
    );
  });

  it("ptr path and Float32Array path both construct layers", async () => {
    const path = `${root}fixtures/scatter_1k.ipc`;
    let buf: Buffer;
    try {
      buf = readFileSync(path);
    } catch {
      console.warn("skip: run scripts/generate_fixtures.py");
      return;
    }
    const table = tableFromIPC(new Uint8Array(buf));
    const a = await scatterFromArrow(table);
    const b = await scatterFromArrowViaFloat32Arrays(table);
    expect(typeof a.free).toBe("function");
    expect(typeof b.free).toBe("function");
    a.free();
    b.free();
  });

  it("IPC column buffers stay on the same ArrayBuffer after scatterFromArrow", async () => {
    const path = `${root}fixtures/scatter_1k.ipc`;
    let buf: Buffer;
    try {
      buf = readFileSync(path);
    } catch {
      return;
    }
    const table = tableFromIPC(new Uint8Array(buf));
    const before = scatterArrowColumnBuffers(table);
    const layer = await scatterFromArrow(table);
    const after = scatterArrowColumnBuffers(table);
    layer.free();
    expect(after).toEqual(before);
  });
});
