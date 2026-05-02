import { describe, expect, it } from "vitest";
import { sceneBuilder, type Layer } from "../src/index.js";

function scatterPointCount(layer: Layer): number {
  if (layer.kind === "scatter") {
    return layer.opts.positions.length / 2;
  }
  return 0;
}

describe("SceneBuilder", () => {
  it("chains scatter() and records layer count", () => {
    const b = sceneBuilder();
    expect(b.layerCount).toBe(0);
    b.scatter({
      positions: new Float32Array([0, 0, 1, 1]),
      colors: new Float32Array(2 * 4).fill(1),
      sizes: new Float32Array([1, 1]),
    });
    expect(b.layerCount).toBe(1);
    b.scatter({
      positions: new Float32Array(0),
      colors: new Float32Array(0),
      sizes: new Float32Array(0),
    });
    expect(b.layerCount).toBe(2);
  });

  it("throws when colors length does not match positions", () => {
    expect(() =>
      sceneBuilder().scatter({
        positions: new Float32Array([0, 0, 1, 1]),
        colors: new Float32Array(4),
        sizes: new Float32Array([1, 1]),
      }),
    ).toThrow(/colors length/);
  });

  it("throws when sizes length does not match point count", () => {
    expect(() =>
      sceneBuilder().scatter({
        positions: new Float32Array([0, 0, 1, 1]),
        colors: new Float32Array(2 * 4).fill(1),
        sizes: new Float32Array([1]),
      }),
    ).toThrow(/sizes length/);
  });

  it("narrows Layer by kind", () => {
    const layer: Layer = {
      kind: "scatter",
      opts: {
        positions: new Float32Array([0, 0]),
        colors: new Float32Array(4),
        sizes: new Float32Array([2]),
      },
    };
    expect(scatterPointCount(layer)).toBe(1);
  });
});
