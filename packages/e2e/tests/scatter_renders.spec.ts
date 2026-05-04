import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { expect, test } from "@playwright/test";
import pixelmatch from "pixelmatch";
import { PNG } from "pngjs";

import { urlForProject } from "./_url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

test("scatter 100 points renders correctly", async ({ page }, testInfo) => {
  await page.goto(
    urlForProject(testInfo, { scenario: "scatter_100", seed: 42 }),
  );

  await page.waitForFunction(() => {
    const s = (
      window as unknown as {
        phantomatStats?: { ttfr: number };
      }
    ).phantomatStats;
    return s != null && Number.isFinite(s.ttfr);
  });

  const screenshot = await page.locator("canvas").screenshot({ type: "png" });
  const baselinePath = path.join(
    __dirname,
    "screenshots",
    testInfo.project.name,
    "scatter_100.png",
  );

  if (process.env.UPDATE_BASELINES === "1") {
    fs.mkdirSync(path.dirname(baselinePath), { recursive: true });
    fs.writeFileSync(baselinePath, screenshot);
    test.info().annotations.push({
      type: "note",
      description: `wrote baseline ${baselinePath}`,
    });
    return;
  }

  expect(
    fs.existsSync(baselinePath),
    `missing baseline ${baselinePath} — run UPDATE_BASELINES=1 pnpm --filter e2e test`,
  ).toBeTruthy();

  const baseline = PNG.sync.read(fs.readFileSync(baselinePath));
  const actual = PNG.sync.read(screenshot as Buffer);
  expect(actual.width, "canvas width").toBe(baseline.width);
  expect(actual.height, "canvas height").toBe(baseline.height);

  const diff = new PNG({ width: baseline.width, height: baseline.height });
  const numDiff = pixelmatch(
    baseline.data,
    actual.data,
    diff.data,
    baseline.width,
    baseline.height,
    { threshold: 0.1 },
  );

  const frac = numDiff / (baseline.width * baseline.height);
  expect(frac, `pixel diff fraction (${numDiff} px)`).toBeLessThan(0.01);
});
