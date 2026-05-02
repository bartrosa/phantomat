import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";

import { expect, test } from "@playwright/test";

import budgets from "../budgets.json" assert { type: "json" };

function ttfrBudget(projectName: string): number {
  const t = budgets.ttfr_1m_pts as Record<string, number>;
  const v = t[projectName];
  if (v == null) throw new Error(`no ttfr budget for ${projectName}`);
  return v;
}

function frameTimeBudget(projectName: string): number {
  const t = budgets.frame_time_1m_pts as Record<string, number>;
  return t[projectName] ?? t["_"] ?? 33;
}

function heapBudget(projectName: string): number {
  const t = budgets.js_heap_mb_1m_pts as Record<string, number>;
  return t[projectName] ?? t["_"] ?? 200;
}

test.describe("Performance budgets", () => {
  const framesParam = process.env.CI ? "10" : "60";

  test("1M scatter TTFR", async ({ page }, testInfo) => {
    await page.goto(`/?scenario=scatter_1m&seed=42&frames=0`);
    await page.waitForFunction(() => {
      const s = (
        window as unknown as {
          phantomatStats?: { ttfr: number };
        }
      ).phantomatStats;
      return s != null && Number.isFinite(s.ttfr);
    });
    const stats = await page.evaluate(() => {
      const s = (
        window as unknown as {
          phantomatStats: { ttfr: number; frameTime: number };
        }
      ).phantomatStats;
      return s;
    });
    const budget = ttfrBudget(testInfo.project.name);
    expect(
      stats.ttfr,
      `TTFR ${stats.ttfr.toFixed(1)}ms vs budget ${budget}ms (${testInfo.project.name})`,
    ).toBeLessThan(budget);
  });

  test("1M scatter frame time (mean over N frames)", async ({
    page,
  }, testInfo) => {
    await page.goto(`/?scenario=scatter_1m&seed=42&frames=${framesParam}`);
    await page.waitForFunction(() => {
      const s = (
        window as unknown as {
          phantomatStats?: { frameTime: number };
        }
      ).phantomatStats;
      return s != null && Number.isFinite(s.frameTime);
    });
    const stats = await page.evaluate(() => {
      const s = (
        window as unknown as {
          phantomatStats: { frameTime: number };
        }
      ).phantomatStats;
      return s;
    });
    const budget = frameTimeBudget(testInfo.project.name);
    expect(
      stats.frameTime,
      `frameTime ${stats.frameTime.toFixed(2)}ms vs budget ${budget}ms (${testInfo.project.name})`,
    ).toBeLessThan(budget);
  });

  test("1M scatter JS heap (Chromium only)", async ({ page, browserName }, testInfo) => {
    test.skip(
      browserName !== "chromium",
      "performance.memory is Chromium-specific",
    );
    await page.goto("/?scenario=scatter_1m&seed=42&frames=0");
    await page.waitForFunction(() => {
      const s = (
        window as unknown as {
          phantomatStats?: { jsHeap?: number };
        }
      ).phantomatStats;
      return s != null && s.jsHeap != null && Number.isFinite(s.jsHeap);
    });
    const mb = await page.evaluate(() => {
      const s = (
        window as unknown as {
          phantomatStats: { jsHeap?: number };
        }
      ).phantomatStats;
      return s.jsHeap ?? 0;
    });
    const budget = heapBudget(testInfo.project.name);
    expect(mb, `heap ${mb.toFixed(1)} MiB vs budget ${budget} MiB`).toBeLessThan(
      budget,
    );
  });

  test("bundle wasm gzip size", () => {
    const here = path.dirname(fileURLToPath(import.meta.url));
    const wasmPath = path.resolve(
      here,
      "../../../crates/phantomat-wasm/pkg/phantomat_wasm_bg.wasm",
    );
    if (!fs.existsSync(wasmPath)) {
      test.skip(true, `missing ${wasmPath} — build wasm first`);
      return;
    }
    const raw = fs.readFileSync(wasmPath);
    const gz = gzipSync(raw).length / 1024;
    const maxKb = budgets.bundle_wasm_kb_gz as number;
    expect(
      gz,
      `wasm gzip ${gz.toFixed(1)} KiB vs budget ${maxKb} KiB`,
    ).toBeLessThanOrEqual(maxKb);
  });
});
