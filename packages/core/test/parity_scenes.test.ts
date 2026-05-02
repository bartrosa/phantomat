import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { describe, expect, it } from "vitest";

const root = fileURLToPath(new URL("../../../", import.meta.url));
const parityRoot = join(root, "tests/differential/python_ts_parity");

const FIXTURES = [
  "scenario_scatter_basic.json",
  "scenario_heatmap_clusters.json",
  "scenario_line_timeseries.json",
  "scenario_bar_categorical.json",
  "scenario_combined_3layers.json",
];

describe("python_ts parity JSON", () => {
  for (const name of FIXTURES) {
    it(name, () => {
      const pyExe = process.env.PYTHON ?? "python3";
      const py = spawnSync(pyExe, [join(parityRoot, "build_scene.py"), name], {
        cwd: join(root, "tests/differential"),
        encoding: "utf-8",
      });
      const js = spawnSync("node", [join(parityRoot, "build_scene.mjs"), name], {
        cwd: join(root, "tests/differential"),
        encoding: "utf-8",
      });
      expect(py.status).toBe(0);
      expect(js.status).toBe(0);
      expect(py.stdout).toBe(js.stdout);
    });
  }
});
