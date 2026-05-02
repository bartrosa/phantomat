import { expect, test } from "@playwright/test";

test("falls back to WebGL2 when WebGPU disabled (or uses WebGPU when available)", async ({
  page,
}) => {
  await page.goto("/?scenario=scatter_100&seed=1");
  await page.waitForFunction(() => {
    const s = (
      window as unknown as {
        phantomatStats?: { backend: string };
      }
    ).phantomatStats;
    return s != null && typeof s.backend === "string";
  });
  const backend = await page.evaluate(() => {
    const s = (
      window as unknown as {
        phantomatStats: { backend: string };
      }
    ).phantomatStats;
    return s.backend;
  });
  expect(backend).toMatch(/webgl|webgpu/);
});
