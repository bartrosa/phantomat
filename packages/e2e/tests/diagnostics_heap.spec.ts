import { expect, test } from "@playwright/test";

import { urlForProject } from "./_url";

test.describe("Heap breakdown for 1M scatter", () => {
  test("heap stability over 1000 frames", async ({ page, browserName }, testInfo) => {
    test.skip(browserName !== "chromium", "performance.memory + CDP only in Chromium");

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
          phantomatScene?: unknown;
          phantomatStats?: { ttfr: number };
        };
        return (
          w.phantomatScene != null &&
          w.phantomatStats != null &&
          Number.isFinite(w.phantomatStats.ttfr) &&
          w.phantomatStats.ttfr > 0
        );
      },
      { timeout: 120_000 },
    );

    const client = await page.context().newCDPSession(page);
    await client.send("HeapProfiler.collectGarbage");

    const heap0 = await page.evaluate(() =>
      (performance as unknown as { memory: { usedJSHeapSize: number } }).memory
        .usedJSHeapSize,
    );

    await page.evaluate(async () => {
      const scene = (
        window as unknown as {
          phantomatScene: { render: () => Promise<void> };
        }
      ).phantomatScene;
      await scene.render();
    });
    await client.send("HeapProfiler.collectGarbage");
    const heap1 = await page.evaluate(() =>
      (performance as unknown as { memory: { usedJSHeapSize: number } }).memory
        .usedJSHeapSize,
    );

    await page.evaluate(async () => {
      const scene = (
        window as unknown as {
          phantomatScene: { render: () => Promise<void> };
        }
      ).phantomatScene;
      for (let i = 0; i < 100; i++) {
        await scene.render();
        await new Promise<void>((r) => requestAnimationFrame(() => r()));
      }
    });
    await client.send("HeapProfiler.collectGarbage");
    const heap100 = await page.evaluate(() =>
      (performance as unknown as { memory: { usedJSHeapSize: number } }).memory
        .usedJSHeapSize,
    );

    await page.evaluate(async () => {
      const scene = (
        window as unknown as {
          phantomatScene: { render: () => Promise<void> };
        }
      ).phantomatScene;
      for (let i = 0; i < 1000; i++) {
        await scene.render();
        if (i % 100 === 0) {
          await new Promise<void>((r) => requestAnimationFrame(() => r()));
        }
      }
    });
    await client.send("HeapProfiler.collectGarbage");
    const heap1000 = await page.evaluate(() =>
      (performance as unknown as { memory: { usedJSHeapSize: number } }).memory
        .usedJSHeapSize,
    );

    const fmt = (n: number) => `${(n / 1024 / 1024).toFixed(1)} MiB`;
    console.log(
      `[HEAP] heap0=${fmt(heap0)} heap1=${fmt(heap1)} heap100=${fmt(heap100)} heap1000=${fmt(heap1000)}`,
    );
    const growth = heap1 > 0 ? (heap1000 - heap1) / heap1 : 0;
    console.log(`[HEAP] growth heap1→heap1000: ${(growth * 100).toFixed(1)}%`);

    expect(growth, "heap growth after 1000 frames vs post-first-render baseline").toBeLessThan(
      0.1,
    );
  });
});
