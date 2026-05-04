import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { test } from "@playwright/test";

import { urlForProject } from "./_url";

test("collect adapter info per browser", async ({ page }, testInfo) => {
  await page.goto(
    urlForProject(testInfo, { scenario: "scatter_100", seed: 42, frames: 0 }),
  );
  await page.waitForFunction(
    () => {
      const w = window as unknown as {
        phantomatStats?: { adapter?: unknown };
      };
      return w.phantomatStats?.adapter != null;
    },
    { timeout: 60_000 },
  );
  const info = await page.evaluate(() => {
    const w = window as unknown as {
      phantomatStats?: Record<string, unknown>;
    };
    return w.phantomatStats ?? {};
  });

  const outDir = path.join(
    path.dirname(fileURLToPath(import.meta.url)),
    "..",
    "test-results",
    "diagnostics",
  );
  fs.mkdirSync(outDir, { recursive: true });
  const file = path.join(outDir, `${testInfo.project.name}.json`);
  fs.writeFileSync(file, JSON.stringify(info, null, 2));

  const adapter = (info as { adapter?: Record<string, unknown> }).adapter;
  const rich = (info as { richAdapter?: Record<string, unknown> }).richAdapter;
  const preInit = (info as { preInit?: Record<string, unknown> }).preInit;
  console.log(
    `[DIAGNOSTIC] ${testInfo.project.name}:`,
    JSON.stringify(adapter, null, 2),
  );
  console.log(`[DIAGNOSTIC] ${testInfo.project.name} preInit:`, JSON.stringify(preInit, null, 2));
  console.log(`[DIAGNOSTIC] ${testInfo.project.name} richAdapter:`, JSON.stringify(rich, null, 2));
});
