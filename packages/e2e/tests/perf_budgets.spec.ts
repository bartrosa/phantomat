import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";

import { expect, test } from "@playwright/test";
import type { TestInfo } from "@playwright/test";

import budgets from "../budgets.json" assert { type: "json" };

import { urlForProject } from "./_url";

function shouldSkipPerfBudgets(testInfo: TestInfo): boolean {
  const m = testInfo.project.metadata as
    | { perfBudgetsApply?: boolean }
    | undefined;
  return m?.perfBudgetsApply === false;
}

function ttfrBudget(projectName: string): number {
  const t = budgets.ttfr_1m_pts as Record<string, number>;
  const v = t[projectName] ?? t["_"];
  if (v == null) throw new Error(`no ttfr budget for ${projectName}`);
  return v;
}

function frameTimeBudget(projectName: string): number {
  const t = budgets.frame_time_1m_pts as Record<string, number>;
  const v = t[projectName];
  if (v == null) throw new Error(`no frame-time budget for ${projectName}`);
  return v;
}

function heapBudget(projectName: string): number {
  const t = budgets.js_heap_mb_1m_pts as Record<string, number>;
  const v = t[projectName] ?? t["_"];
  if (v == null) throw new Error(`no heap budget for ${projectName}`);
  return v;
}

test.describe("Performance budgets (GPU)", () => {
  test("1M scatter TTFR (cold)", async ({ page }, testInfo) => {
    test.skip(
      shouldSkipPerfBudgets(testInfo),
      "GPU perf budgets disabled for this Playwright project — see README",
    );
    await page.goto(
      urlForProject(testInfo, {
        scenario: "scatter_1m",
        seed: 42,
        frames: 0,
      }),
    );
    await page.waitForFunction(
      () => {
        const w = window as unknown as {
          phantomatStats?: { ttfr: number };
        };
        return (
          w.phantomatStats != null &&
          Number.isFinite(w.phantomatStats.ttfr) &&
          w.phantomatStats.ttfr > 0
        );
      },
      { timeout: 120_000 },
    );
    const ttfr = await page.evaluate(() => {
      const w = window as unknown as { phantomatStats: { ttfr: number } };
      return w.phantomatStats.ttfr;
    });
    console.log(`[METRIC] project=${testInfo.project.name} ttfr_ms=${ttfr.toFixed(2)}`);
    const budget = ttfrBudget(testInfo.project.name);
    expect(
      ttfr,
      `TTFR ${ttfr.toFixed(1)}ms vs budget ${budget}ms (${testInfo.project.name})`,
    ).toBeLessThan(budget);
  });

  test("1M scatter steady-state frame time (median of 60 warmed frames)", async ({
    page,
  }, testInfo) => {
    test.skip(
      shouldSkipPerfBudgets(testInfo),
      "GPU perf budgets disabled for this Playwright project — see README",
    );
    await page.goto(
      urlForProject(testInfo, {
        scenario: "scatter_1m",
        seed: 42,
        frames: 0,
      }),
    );
    await page.waitForFunction(
      () => {
        const w = window as unknown as {
          phantomatStats?: { ttfr: number };
          phantomatScene?: unknown;
        };
        return (
          w.phantomatStats != null &&
          Number.isFinite(w.phantomatStats.ttfr) &&
          w.phantomatStats.ttfr > 0 &&
          w.phantomatScene != null
        );
      },
      { timeout: 120_000 },
    );

    const { median, p95 } = await page.evaluate(async () => {
      const scene = (
        window as unknown as {
          phantomatScene: { render: () => Promise<void> };
        }
      ).phantomatScene;
      for (let i = 0; i < 10; i++) {
        await scene.render();
      }
      const times: number[] = [];
      for (let i = 0; i < 60; i++) {
        const t0 = performance.now();
        await scene.render();
        times.push(performance.now() - t0);
      }
      const sorted = [...times].sort((a, b) => a - b);
      const median = sorted[Math.floor(sorted.length / 2)] ?? 0;
      const p95 = sorted[Math.floor(sorted.length * 0.95)] ?? 0;
      return { median, p95 };
    });

    console.log(
      `[PERF] ${testInfo.project.name} median=${median.toFixed(2)}ms p95=${p95.toFixed(2)}ms`,
    );
    console.log(
      `[METRIC] project=${testInfo.project.name} frame_median_ms=${median.toFixed(2)} frame_p95_ms=${p95.toFixed(2)}`,
    );

    const budget = frameTimeBudget(testInfo.project.name);
    expect(
      median,
      `median frame ${median.toFixed(2)}ms vs budget ${budget}ms (${testInfo.project.name})`,
    ).toBeLessThan(budget);
  });

  test("1M scatter JS heap (Chromium only)", async ({ page, browserName }, testInfo) => {
    test.skip(
      shouldSkipPerfBudgets(testInfo),
      "GPU perf budgets disabled for this Playwright project — see README",
    );
    test.skip(
      browserName !== "chromium",
      "performance.memory is Chromium-specific",
    );
    await page.goto(
      urlForProject(testInfo, {
        scenario: "scatter_1m",
        seed: 42,
        frames: 0,
      }),
    );
    await page.waitForFunction(
      () => {
        const w = window as unknown as {
          phantomatStats?: { jsHeap?: number };
        };
        return (
          w.phantomatStats != null &&
          w.phantomatStats.jsHeap != null &&
          Number.isFinite(w.phantomatStats.jsHeap)
        );
      },
      { timeout: 120_000 },
    );
    const mb = await page.evaluate(() => {
      const w = window as unknown as {
        phantomatStats: { jsHeap?: number };
      };
      return w.phantomatStats.jsHeap ?? 0;
    });
    console.log(`[METRIC] project=${testInfo.project.name} heap_mb=${mb.toFixed(2)}`);
    const budget = heapBudget(testInfo.project.name);
    expect(mb, `heap ${mb.toFixed(1)} MiB vs budget ${budget} MiB`).toBeLessThan(
      budget,
    );
  });
});

test.describe("Performance budgets (bundle)", () => {
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
