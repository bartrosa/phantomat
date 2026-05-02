#!/usr/bin/env node
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = dirname(fileURLToPath(import.meta.url));

function stableStringify(obj) {
  if (obj === null || typeof obj !== "object") {
    return JSON.stringify(obj);
  }
  if (Array.isArray(obj)) {
    return `[${obj.map(stableStringify).join(",")}]`;
  }
  const keys = Object.keys(obj).sort();
  return `{${keys.map((k) => `${JSON.stringify(k)}:${stableStringify(obj[k])}`).join(",")}}`;
}

function sceneFromFixture(json) {
  if (json.version !== 1) throw new Error("version");
  const layersOut = [];
  for (const layer of json.layers) {
    const t = layer.type;
    if (t === "scatter") {
      const seed = layer.seed;
      const n = layer.points;
      const h = createHash("sha256")
        .update(`scatter-${seed}-${n}`)
        .digest("hex")
        .slice(0, 16);
      layersOut.push({ type: "scatter", digest: h, n, seed });
    } else if (t === "heatmap") {
      const seed = layer.seed;
      const n = layer.points;
      const h = createHash("sha256")
        .update(`heatmap-${seed}-${n}`)
        .digest("hex")
        .slice(0, 16);
      layersOut.push({
        type: "heatmap",
        digest: h,
        bins: layer.bins,
        n,
        seed,
      });
    } else if (t === "line_stub") {
      layersOut.push({ type: "line_stub", n: layer.n });
    } else if (t === "bar_stub") {
      layersOut.push({ type: "bar_stub", categories: layer.categories });
    } else throw new Error(t);
  }
  return { layers: layersOut, version: 1 };
}

const name = process.argv[2];
if (!name) {
  console.error("usage: build_scene.mjs <fixture.json>");
  process.exit(2);
}
const fixturePath = join(root, "fixtures", name);
const json = JSON.parse(readFileSync(fixturePath, "utf8"));
const scene = sceneFromFixture(json);
process.stdout.write(stableStringify(scene) + "\n");
