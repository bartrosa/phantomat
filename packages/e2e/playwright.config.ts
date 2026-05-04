import { platform } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, devices } from "@playwright/test";

/** Headless Chrome needs an explicit ANGLE backend for reliable WebGL2 (wgpu GL); not related to WebGPU flags. */
const chromiumWebGlAngleArgs =
  platform() === "darwin" ? ["--use-angle=metal"] : ["--use-angle=swiftshader"];

const isCI = Boolean(process.env.CI);
/** Opt-in WebKit project (headless WebKit + wgpu/WASM often hangs before first frame). */
const includeWebKit = process.env.E2E_WEBKIT === "1";
/** Monorepo root (contains `pnpm-lock.yaml`). */
const repoRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
);

export default defineConfig({
  testDir: "tests",
  fullyParallel: true,
  forbidOnly: isCI,
  /* Retries hide steady-state perf regressions; cold/warm split + median metrics replace flake retries. */
  retries: 0,
  workers: isCI ? 1 : undefined,
  reporter: [["list"], ["html", { open: "never" }]],
  timeout: 300_000,
  expect: { timeout: 60_000 },
  use: {
    trace: "on-first-retry",
    baseURL: "http://localhost:4173",
  },
  projects: [
    {
      name: "chromium-webgpu",
      use: {
        ...devices["Desktop Chrome"],
        launchOptions: {
          args: ["--enable-unsafe-webgpu"],
        },
      },
    },
    {
      name: "chromium-webgl",
      use: {
        ...devices["Desktop Chrome"],
        launchOptions: {
          args: chromiumWebGlAngleArgs,
        },
      },
    },
    {
      name: "firefox-webgpu",
      use: { ...devices["Desktop Firefox"] },
      metadata: {
        perfBudgetsApply: false,
        perfBudgetsReason:
          "Firefox WebGPU/WebGL on hosted runners is often software-throttled; TTFR/frame/heap budgets run on chromium-* only — see packages/e2e/README.md",
      },
    },
    {
      name: "firefox-webgl",
      use: {
        ...devices["Desktop Firefox"],
        firefoxUserPrefs: {
          "dom.webgpu.enabled": false,
        },
      } as (typeof devices)["Desktop Firefox"] & {
        firefoxUserPrefs: Record<string, string | number | boolean>;
      },
      metadata: {
        perfBudgetsApply: false,
        perfBudgetsReason:
          "Firefox WebGPU/WebGL on hosted runners is often software-throttled; TTFR/frame/heap budgets run on chromium-* only — see packages/e2e/README.md",
      },
    },
    ...(includeWebKit
      ? [
          {
            name: "webkit",
            use: { ...devices["Desktop Safari"] },
          },
        ]
      : []),
  ],
  webServer: {
    command: "npx vite preview --port 4173 --strictPort",
    cwd: path.join(repoRoot, "examples", "web"),
    url: "http://localhost:4173",
    reuseExistingServer: !isCI,
    timeout: 300_000,
  },
});
